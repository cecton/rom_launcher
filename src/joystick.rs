use sdl2::joystick::Joystick;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct JoystickInfo {
    pub instance_id: i32,
    pub guid: JoystickGuid,
    // NOTE: index is the second unique identifier for guid when there are collisions, mostly 0
    pub index: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct JoystickGuid([u8; 16]);

impl JoystickInfo {
    pub fn new(joystick: &Joystick, index: usize) -> JoystickInfo {
        let instance_id = joystick.instance_id();
        let guid = JoystickGuid(joystick.guid().raw().data);

        JoystickInfo {
            instance_id,
            guid,
            index,
        }
    }
}
