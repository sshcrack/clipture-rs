pub fn to_internal_res<T, E: ToString>(r: Result<T, E>) -> Result<T, rspc::Error> {
    r.map_err(|e| rspc::Error::new(rspc::ErrorCode::InternalServerError, e.to_string()))
}
