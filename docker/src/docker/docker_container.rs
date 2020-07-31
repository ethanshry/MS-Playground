use std::fmt;
/// This is a reduction of bollard's ContainerSummaryInner
pub struct DockerContainer {
    /// The ID of this container
    pub id: String,

    /// The names that this container has been given
    pub name: String,

    /// The name of the image used when creating this container
    pub image: Option<String>,

    /// The ID of the image that this container was created from
    pub image_id: Option<String>,

    /// When the container was created
    pub created: Option<i64>,

    /// The ports exposed by this container
    pub ports: Option<Vec<i64>>,

    /// The state of this container (e.g. `Exited`)
    pub state: Option<String>,

    /// Additional human-readable status of this container (e.g. `Exit 0`)
    pub status: Option<String>,
}

impl DockerContainer {
    pub fn new(
        id: String,
        name: String,
        image: Option<String>,
        image_id: Option<String>,
        created: Option<i64>,
        ports: Option<Vec<i64>>,
        state: Option<String>,
        status: Option<String>,
    ) -> DockerContainer {
        DockerContainer {
            id: id,
            name: name,
            image: image,
            image_id: image_id,
            created: created,
            ports: ports,
            state: state,
            status: status,
        }
    }
}

impl fmt::Debug for DockerContainer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DockerContainer")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("image", &self.image)
            .field("image_id", &self.image_id)
            .field("created", &self.created)
            .field("ports", &self.ports)
            .field("state", &self.state)
            .field("status", &self.status)
            .finish()
    }
}
