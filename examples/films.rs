pub struct ListFilmsRows {
    pub film_id: i32,
    pub title: String,
}
pub async fn list_films(
    c: &impl tokio_postgres::GenericClient,
) -> Result<Vec<ListFilmsRows>, tokio_postgres::Error> {
    c.query("SELECT film_id, title FROM films", &[])
        .await
        .map(|rs| {
            rs.into_iter()
                .map(|r| ListFilmsRows {
                    film_id: r.get(0),
                    title: r.get(1),
                })
                .collect()
        })
}

pub struct FindFilmParams {
    pub eq_film_id: i32,
}
pub struct FindFilmRows {
    pub film_id: i32,
    pub title: String,
}
pub async fn find_film(
    c: &impl tokio_postgres::GenericClient,
    p: FindFilmParams,
) -> Result<Vec<FindFilmRows>, tokio_postgres::Error> {
    c.query("SELECT film_id, title FROM films WHERE film_id = $1", &[&p.eq_film_id])
        .await
        .map(|rs| {
            rs.into_iter()
                .map(|r| FindFilmRows {
                    film_id: r.get(0),
                    title: r.get(1),
                })
                .collect()
        })
}

pub struct CreateFilmParams {
    pub title: String,
}
pub struct CreateFilmRows {
    pub film_id: i32,
}
pub async fn create_film(
    c: &impl tokio_postgres::GenericClient,
    p: CreateFilmParams,
) -> Result<Vec<CreateFilmRows>, tokio_postgres::Error> {
    c.query("INSERT INTO films (title) VALUES ($1) RETURNING film_id", &[&p.title])
        .await
        .map(|rs| {
            rs.into_iter()
                .map(|r| CreateFilmRows {
                    film_id: r.get(0),
                })
                .collect()
        })
}

pub struct UpdateUserParams {
    pub eq_film_id: i32,
    pub set_title: String,
}
pub struct UpdateUserRows {
    pub film_id: i32,
    pub title: String,
}
pub async fn update_user(
    c: &impl tokio_postgres::GenericClient,
    p: UpdateUserParams,
) -> Result<Vec<UpdateUserRows>, tokio_postgres::Error> {
    c.query(
            "UPDATE films SET title = $2 WHERE film_id = $1 RETURNING film_id, title",
            &[&p.eq_film_id, &p.set_title],
        )
        .await
        .map(|rs| {
            rs.into_iter()
                .map(|r| UpdateUserRows {
                    film_id: r.get(0),
                    title: r.get(1),
                })
                .collect()
        })
}

pub struct DeleteUserParams {
    pub eq_film_id: i32,
}
pub struct DeleteUserRows {
    pub film_id: i32,
    pub title: String,
}
pub async fn delete_user(
    c: &impl tokio_postgres::GenericClient,
    p: DeleteUserParams,
) -> Result<Vec<DeleteUserRows>, tokio_postgres::Error> {
    c.query(
            "DELETE FROM films WHERE film_id = $1 RETURNING film_id, title",
            &[&p.eq_film_id],
        )
        .await
        .map(|rs| {
            rs.into_iter()
                .map(|r| DeleteUserRows {
                    film_id: r.get(0),
                    title: r.get(1),
                })
                .collect()
        })
}

// The main is not autogenerated, but is needed to example folder to compile
fn main() {}