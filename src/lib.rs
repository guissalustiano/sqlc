use pg_query::{
    protobuf::{a_const::Val, JoinType},
    Node, NodeEnum,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Int,
    String,
    Bytes,
    Boolean,
    Float,
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnData {
    pub type_: Type,
    pub nullable: bool,
}

impl ColumnData {
    fn string() -> Self {
        ColumnData {
            type_: Type::String,
            nullable: false,
        }
    }
    fn int() -> Self {
        ColumnData {
            type_: Type::Int,
            nullable: false,
        }
    }
    fn int_nullable() -> Self {
        ColumnData {
            type_: Type::Int,
            nullable: true,
        }
    }
    fn bytes() -> Self {
        ColumnData {
            type_: Type::Bytes,
            nullable: false,
        }
    }
    fn boolean() -> Self {
        ColumnData {
            type_: Type::Boolean,
            nullable: false,
        }
    }
    fn float() -> Self {
        ColumnData {
            type_: Type::Float,
            nullable: false,
        }
    }
    fn null() -> Self {
        ColumnData {
            type_: Type::Null,
            nullable: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Column<'a> {
    pub name: &'a str,
    pub data: ColumnData,
}

#[derive(Debug, Clone)]
pub struct Table<'a> {
    pub name: &'a str,
    pub columns: &'a [Column<'a>],
}

#[derive(Debug)]
pub struct Catalog<'a> {
    pub tables: &'a [Table<'a>],
}

impl Catalog<'_> {
    fn find_table(&self, t_name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name == t_name)
    }

    fn find_type(&self, t_name: &str, c_name: &str) -> Option<ColumnData> {
        self.find_table(t_name)?
            .columns
            .iter()
            .find(|c| c.name == c_name)
            .map(|c| c.data)
    }

    fn as_ctx(&self) -> Ctx {
        self.tables
            .iter()
            .flat_map(|t| {
                t.columns
                    .iter()
                    .map(|c| CtxEntry::new(t.name, c.name, c.data))
            })
            .collect()
    }
}

#[cfg(test)]
fn parse(sql: &str) -> NodeEnum {
    pg_query::parse(sql)
        .unwrap()
        .protobuf
        .stmts
        .first()
        .unwrap()
        .stmt
        .as_ref()
        .unwrap()
        .node
        .as_ref()
        .unwrap()
        .clone()
}

type Ctx<'a> = Vec<CtxEntry<'a>>;

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct CtxEntry<'a> {
    pub table: Option<&'a str>,
    pub column: Option<&'a str>,
    pub data: ColumnData,
}
impl<'a> CtxEntry<'a> {
    pub(crate) fn new(table: &'a str, column: &'a str, data: ColumnData) -> Self {
        Self {
            table: Some(table),
            column: Some(column),
            data,
        }
    }

    pub(crate) fn new_anonymous(data: ColumnData) -> Self {
        Self {
            table: None,
            column: None,
            data,
        }
    }
}

pub(crate) fn solve_from<'a>(sys_ctx: Ctx<'a>, from: Vec<Node>) -> Ctx<'a> {
    from.iter()
        .fold(Vec::with_capacity(sys_ctx.len()), |mut ctx, n| {
            match n.node.as_ref().expect("from.node") {
                NodeEnum::RangeVar(rv) => {
                    ctx.extend(sys_ctx.iter().filter(|e| e.table == Some(&rv.relname)));
                    ctx
                }
                NodeEnum::JoinExpr(je) => {
                    let NodeEnum::RangeVar(larg) = je
                        .larg
                        .as_ref()
                        .expect("larg")
                        .node
                        .as_ref()
                        .expect("larg.node")
                    else {
                        unimplemented!("larg");
                    };
                    let NodeEnum::RangeVar(rarg) = je
                        .rarg
                        .as_ref()
                        .expect("rarg")
                        .node
                        .as_ref()
                        .expect("rarg.node")
                    else {
                        unimplemented!("rarg");
                    };

                    ctx.extend(sys_ctx.iter().filter(|e| e.table == Some(&larg.relname)));

                    match je.jointype() {
                        JoinType::JoinInner => {
                            ctx.extend(sys_ctx.iter().filter(|e| e.table == Some(&rarg.relname)))
                        }
                        JoinType::JoinLeft => ctx.extend(
                            sys_ctx
                                .iter()
                                .filter(|e| e.table == Some(&rarg.relname))
                                .map(|e| CtxEntry {
                                    data: ColumnData {
                                        nullable: true,
                                        ..e.data
                                    },
                                    ..*e
                                }),
                        ),
                        _ => unimplemented!("join type"),
                    };

                    ctx
                }
                _ => unimplemented!("relname"),
            }
        })
}

pub(crate) fn solve_type<'a>(ctg: &'a Catalog, stmt: NodeEnum) -> Ctx<'a> {
    match stmt {
        NodeEnum::SelectStmt(s) => {
            let ctx = solve_from(ctg.as_ctx(), s.from_clause);

            s.target_list
                .iter()
                .map(|target| {
                    let NodeEnum::ResTarget(target) = target.node.as_ref().unwrap() else {
                        unimplemented!("target")
                    };
                    let target = target.val.as_ref().unwrap().node.as_ref().unwrap();

                    match target {
                        NodeEnum::ColumnRef(cr) => {
                            let &[t_name, c_name] = &cr
                                .fields
                                .iter()
                                .map(|f| match f.node.as_ref().unwrap() {
                                    NodeEnum::String(pg_query::protobuf::String { sval }) => sval,
                                    _ => unimplemented!("column ref"),
                                })
                                .collect::<Vec<_>>()[..]
                            else {
                                panic!("invalid name, use table.column")
                            };

                            // find type
                            *ctx.iter()
                                .find(|e| {
                                    e.table.as_deref() == Some(&t_name)
                                        && e.column.as_deref() == Some(c_name)
                                })
                                .expect("selected table/name not found")
                        }
                        NodeEnum::AConst(c) => match c.val.as_ref() {
                            Some(Val::Ival(_)) => CtxEntry::new_anonymous(ColumnData::int()),
                            Some(Val::Fval(_)) => CtxEntry::new_anonymous(ColumnData::float()),
                            Some(Val::Boolval(_)) => CtxEntry::new_anonymous(ColumnData::boolean()),
                            Some(Val::Sval(_)) => CtxEntry::new_anonymous(ColumnData::string()),
                            Some(Val::Bsval(_)) => CtxEntry::new_anonymous(ColumnData::bytes()),
                            None => CtxEntry::new_anonymous(ColumnData::null()),
                        },
                        _ => unimplemented!("column"),
                    }
                })
                .collect()
        }
        _ => unimplemented!("stmt"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use super::*;

    type C = ColumnData;
    fn tables_fixture() -> Catalog<'static> {
        /*
        create table x(a text not null, b int);
        create table y(c int not null, d bytea not null);
        */

        Catalog {
            tables: vec![
                Table {
                    name: "x",
                    columns: vec![
                        Column {
                            name: "a",
                            data: ColumnData::string(),
                        },
                        Column {
                            name: "b",
                            data: ColumnData::int_nullable(),
                        },
                    ]
                    .leak(),
                },
                Table {
                    name: "y",
                    columns: vec![
                        Column {
                            name: "c",
                            data: ColumnData::int(),
                        },
                        Column {
                            name: "d",
                            data: ColumnData::bytes(),
                        },
                    ]
                    .leak(),
                },
                Table {
                    name: "w",
                    columns: vec![Column {
                        name: "e",
                        data: ColumnData::int(),
                    }]
                    .leak(),
                },
            ]
            .leak(),
        }
    }

    #[test]
    fn resolve_simple_select() {
        let ctl = tables_fixture();

        let ast = parse("SELECT x.a, x.b FROM x");
        let expected = vec![
            CtxEntry::new("x", "a", C::string()),
            CtxEntry::new("x", "b", C::int_nullable()),
        ];

        assert_eq!(solve_type(&ctl, ast), expected);
    }

    #[test]
    fn resolve_simple_select_with_literal() {
        let ctl = tables_fixture();

        let ast = parse("SELECT y.d, 1, '123', NULL FROM y");
        let expected = vec![
            CtxEntry::new("y", "d", C::bytes()),
            CtxEntry::new_anonymous(C::int()),
            CtxEntry::new_anonymous(C::string()),
            CtxEntry::new_anonymous(C::null()),
        ];

        assert_eq!(solve_type(&ctl, ast), expected);
    }

    #[test]
    #[should_panic(expected = "selected table/name not found")]
    fn resolve_based_on_from() {
        let ctl = tables_fixture();

        // x is not present on from clause
        let ast = parse("SELECT x.a FROM y");

        solve_type(&ctl, ast);
    }

    #[test]
    fn left_join_is_marked_as_null() {
        let ctl = tables_fixture();

        let ast = parse("SELECT x.a, y.c FROM x LEFT JOIN y ON x.b = y.c");
        let expected = vec![
            CtxEntry::new("x", "a", C::string()),
            CtxEntry::new("y", "c", C::int_nullable()),
        ];

        assert_eq!(solve_type(&ctl, ast), expected);
    }

    #[test]
    fn inner_join_is_not_marked_as_null() {
        let ctl = tables_fixture();

        let ast = parse("SELECT x.a, y.c FROM x INNER JOIN y ON x.b = y.c");
        let expected = vec![
            CtxEntry::new("x", "a", C::string()),
            CtxEntry::new("y", "c", C::int()),
        ];

        assert_eq!(solve_type(&ctl, ast), expected);
    }

    #[test]
    #[ignore]
    fn multiple_join_works() {
        let ctl = tables_fixture();

        let ast =
            parse("SELECT x.a, y.c, w.e FROM x LEFT JOIN y ON x.b = y.c INNER JOIN w ON x.b = w.e");
        let expected = vec![
            CtxEntry::new("x", "a", C::string()),
            CtxEntry::new("y", "c", C::int_nullable()),
            CtxEntry::new("w", "e", C::int()),
        ];

        assert_eq!(solve_type(&ctl, ast), expected);
    }
}
