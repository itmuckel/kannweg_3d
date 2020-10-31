use rg3d::sound::buffer::{DataSource, SoundBuffer};
use rg3d::sound::context::Context;
use rg3d::sound::pool::Handle;
use rg3d::sound::source::generic::GenericSourceBuilder;
use rg3d::sound::source::{SoundSource, Status};

pub fn start_ambient_sound() {
    // Initialize new sound context with default output device.
    let context = Context::new().unwrap();

    // Load sound buffer.
    let humming_buffer =
        SoundBuffer::new_streaming(DataSource::from_file("assets/humming.ogg").unwrap()).unwrap();

    // Create flat source (without spatial effects) using that buffer.
    let source = GenericSourceBuilder::new(humming_buffer)
        .with_status(Status::Playing)
        .with_looping(true)
        .with_gain(0.1)
        .build_source()
        .unwrap();

    // Each sound sound must be added to context, context takes ownership on source
    // and returns pool handle to it by which it can be accessed later on if needed.
    let _source_handle: Handle<SoundSource> = context.lock().unwrap().add_source(source);
}
