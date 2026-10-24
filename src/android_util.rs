use jni::{*, errors::{Result as JniResult}, objects::JObject, sys::jobject};

pub use winit::platform::android::{*, activity::*};


// helper functions
pub fn in_jvm_env<F>(app_ref: &AndroidApp, handler: F)
    where F: FnOnce(&mut Env, AndroidApp) -> JniResult<()> + Send + 'static,
{
    let app = app_ref.clone();
    app_ref.run_on_java_main_thread(Box::new(move || {
        let jvm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _) };
        if let Err(err) = jvm.attach_current_thread(|env| handler(env, app)) {
            log::error!("jvm env handler failed: {err:?}");
        }
    }));
}

// getter
pub fn get_activity<'a>(env: &mut jni::Env<'a>, app: &AndroidApp) -> JObject<'a> {
    let activity = app.activity_as_ptr() as jobject;
    unsafe { JObject::from_raw(env, activity) }
}

pub fn get_window<'a>(env: &mut jni::Env<'a>, app: &AndroidApp) -> JniResult<JObject<'a>> {
    let activity = get_activity(env, app);
    env.call_method(&activity, jni_str!("getWindow"), jni_sig!("()Landroid/view/Window;"), &[])?.l()
}

pub fn get_decor<'a>(env: &mut jni::Env<'a>, window: &JObject) -> JniResult<JObject<'a>> {
    env.call_method(&window, jni_str!("getDecorView"), jni_sig!("()Landroid/view/View;"), &[])?.l()
}

pub fn get_window_decor<'a>(env: &mut jni::Env<'a>, app: &AndroidApp) -> JniResult<(JObject<'a>, JObject<'a>)> {
    let window = get_window(env, app)?;
    let decor = get_decor(env, &window)?;
    Ok((window, decor))
}

// fun
pub fn add_window_flags<'a>(env: &mut jni::Env<'a>, window: &JObject, add: bool, flags: i32) -> JniResult<()> {
    let method = if add { jni_str!("addFlags") } else { jni_str!("clearFlags") };
    env.call_method(window, method, jni_sig!("(I)V"), &[JValue::Int(flags)])?;
    Ok(())
}

pub fn get_ui_flags<'a>(env: &mut jni::Env<'a>, decor: &JObject) -> JniResult<i32> {
    env.call_method(&decor, jni_str!("getSystemUiVisibility"), jni_sig!("()I"), &[])?.i()
}

pub fn set_ui_flags<'a>(env: &mut jni::Env<'a>, decor: &JObject, flags: i32) -> JniResult<()> {
    env.call_method(&decor, jni_str!("setSystemUiVisibility"), jni_sig!("(I)V"), &[JValue::Int(flags)])?;
    Ok(())
}

pub fn add_ui_flags<'a>(env: &mut jni::Env<'a>, decor: &JObject, add: bool, flags: i32) -> JniResult<()> {
    let current = get_ui_flags(env, decor)?;
    let set_flags = match add {
        true => current | flags,
        false => current & !flags,
    };
    set_ui_flags(env, decor, set_flags)?;
    Ok(())
}


// suspend the app
pub fn suspend_app(app: &AndroidApp) {
    in_jvm_env(app, move |env, app| {
        let activity = get_activity(env, &app);
        env.call_method(&activity, jni_str!("moveTaskToBack"), jni_sig!("(Z)Z"), &[JValue::Bool(true)])?;
        Ok(())
    });
}

// fullscreen
pub const FULLSCREEN_FLAGS: i32 = 0x00000400; // FLAG_FULLSCREEN

pub const HIDE_UI_FLAGS: i32 =
    0x00000002 | // SYSTEM_UI_FLAG_HIDE_NAVIGATION
    0x00000004 | // SYSTEM_UI_FLAG_FULLSCREEN
    0x00000100 | // SYSTEM_UI_FLAG_LAYOUT_STABLE
    0x00000200 | // SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
    0x00000400 | // SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
    0x00001000   // SYSTEM_UI_FLAG_IMMERSIVE_STICKY
;

pub fn set_fullscreen(app: &AndroidApp, fullscreen: bool) {
    in_jvm_env(app, move |env, app| {
        let (window, decor) = get_window_decor(env, &app)?;
        add_window_flags(env, &window, fullscreen, FULLSCREEN_FLAGS)?;
        add_ui_flags(env, &decor, fullscreen, HIDE_UI_FLAGS)?;
        Ok(())
    });
}

// light status bar
pub const LIGHT_STATUS_BAR_FLAGS: i32 = 0x00002000; // SYSTEM_UI_FLAG_LIGHT_NAVIGATION

pub fn set_light_status_bar(app: &AndroidApp, light: bool) {
    in_jvm_env(app, move |env, app| {
        let (_window, decor) = get_window_decor(env, &app)?;
        add_ui_flags(env, &decor, light, LIGHT_STATUS_BAR_FLAGS)?;
        Ok(())
    });
}

// wake_lock
pub const WAKE_LOCK_FLAGS: i32 = 0x00000080; // KEEP_SCREEN_ON

pub fn set_wake_lock(app: &AndroidApp, lock: bool) {
    in_jvm_env(app, move |env, app| {
        let window = get_window(env, &app)?;
        add_window_flags(env, &window, lock, WAKE_LOCK_FLAGS)?;
        Ok(())
    });
}