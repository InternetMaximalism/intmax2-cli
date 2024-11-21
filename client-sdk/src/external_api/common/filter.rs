#[derive(Debug, Serialize, Deserialize)]
pub struct Filter {
    pub relation: String,
    pub data_field: String,
    pub condition: String,
    pub value: String,
}
