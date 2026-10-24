

use jni::{*, errors::{Result as JniResult}, objects::JObject, sys::jobject};

pub use winit::platform::android::{*, activity::AndroidApp};


pub fn with_window_env<F>(app_ref: &AndroidApp, handler: F)
    where F: FnOnce(&mut Env, &JObject, &JObject) -> JniResult<()> + Send + 'static,
{
    let app = app_ref.clone();
    app_ref.run_on_java_main_thread(Box::new(move || {

        let jvm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _) };

        if let Err(err) = jvm.attach_current_thread(|env| -> JniResult<()> {

            let activity = app.activity_as_ptr() as jobject;
            let activity = unsafe { JObject::from_raw(env, activity) };

            let window = env.call_method(&activity, jni_str!("getWindow"), jni_sig!("()Landroid/view/Window;"), &[])?.l()?;
            let decor = env.call_method(&window, jni_str!("getDecorView"), jni_sig!("()Landroid/view/View;"), &[])?.l()?;

            handler(env, &window, &decor)
        }) {
            log::error!("window handler failed: {err:?}");
        }
    }));
}


// fullscreen handling
const JNI_SIG: signature::MethodSignature = jni_sig!("(I)V");

const WIN_FLAGS: JValue = JValue::Int(
    0x00000400 | // FULLSCREEN
    0x00000100   // LAYOUT_IN_SCREEN
);

const UI_FLAGS: JValue = JValue::Int(
    0x00000002 | // SYSTEM_UI_FLAG_HIDE_NAVIGATION
    0x00000004 | // SYSTEM_UI_FLAG_FULLSCREEN
    0x00000100 | // SYSTEM_UI_FLAG_LAYOUT_STABLE
    0x00000200 | // SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
    0x00000400 | // SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
    0x00001000   // SYSTEM_UI_FLAG_IMMERSIVE_STICKY
);

const WAKE_LOCK_FLAGS: JValue = JValue::Int(0x00000080); // KEEP_SCREEN_ON


pub fn enable_fullscreen(app: &AndroidApp) {
    with_window_env(app, |env, window, decor| {
        env.call_method(window, jni_str!("addFlags"), JNI_SIG, &[WIN_FLAGS])?;
        env.call_method(decor, jni_str!("setSystemUiVisibility"), JNI_SIG, &[UI_FLAGS])?;
        Ok(())
    });
}

pub fn disable_fullscreen(app: &AndroidApp) {
    with_window_env(app, |env, window, decor| {
        env.call_method(window, jni_str!("clearFlags"), JNI_SIG, &[WIN_FLAGS])?;
        env.call_method(decor, jni_str!("setSystemUiVisibility"), JNI_SIG, &[JValue::Int(0)])?;
        Ok(())
    });
}


pub fn enable_wake_lock(app: &AndroidApp) {
    with_window_env(app, |env, window, _decor| {
        env.call_method(window, jni_str!("addFlags"), JNI_SIG, &[WAKE_LOCK_FLAGS])?;
        Ok(())
    });
}

pub fn disable_wake_lock(app: &AndroidApp) {
    with_window_env(app, |env, window, _decor| {
        env.call_method(window, jni_str!("clearFlags"), JNI_SIG, &[WAKE_LOCK_FLAGS])?;
        Ok(())
    });
}