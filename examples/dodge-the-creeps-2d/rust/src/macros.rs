#[macro_export]
macro_rules! node {
    ($self:expr, $path:expr) => {
        $self.base().get_node($path)
    };
}

#[macro_export]
macro_rules! node_as {
    ($self:expr, $path:expr, $type:ty) => {
        $self.base().get_node_as::<$type>($path)
    };
}

// Emulates Godot's $ syntax for node access
#[macro_export]
macro_rules! gd_node {
    ($self:expr, $path:expr) => {
        $self.base().get_node($path)
    };
}

// Emulates Godot's $ syntax with type casting
#[macro_export]
macro_rules! gd_node_as {
    ($self:expr, $path:expr, $type:ty) => {
        $self.base().get_node_as::<$type>($path)
    };
} 