use std::panic::{AssertUnwindSafe, catch_unwind};

use jni::EnvUnowned;
use jni::objects::{JClass, JString, LoaderContext};
use jni::strings::JNIString;
use jni::sys::{jlong, jobject};

use crate::{
    NativeError, NativeResult, add_documents, close_index, commit, commit_and_refresh,
    delete_all_documents, delete_query, delete_term, open_index, refresh, schema_info, search,
};

const EXCEPTION_PACKAGE: &str = "com/rustedbytes/tantivy";
const EXCEPTION_PACKAGE_BINARY: &str = "com.rustedbytes.tantivy";

fn run_native<'env, T>(
    env: &mut jni::Env<'env>,
    action: impl FnOnce(&mut jni::Env<'env>) -> NativeResult<T>,
) -> NativeResult<T> {
    catch_unwind(AssertUnwindSafe(|| action(env))).map_err(|_| NativeError::Panic)?
}

// The exception classes live in the application class loader, which JNI class
// lookups don't reliably reach from arbitrary threads on Android (e.g. the
// thread-context class loader of a coroutine worker may miss app classes), so
// resolve them through the class loader of the calling NativeTantivy class.
fn throw_specific(
    env: &mut jni::Env<'_>,
    caller_class: &JClass<'_>,
    class_name: &str,
    message: &str,
) -> jni::errors::Result<()> {
    let loader = caller_class.get_class_loader(env)?;
    let binary_name: JNIString = format!("{EXCEPTION_PACKAGE_BINARY}.{class_name}").into();
    let exception_class = LoaderContext::Loader(&loader).load_class(env, &binary_name, false)?;
    env.throw_new(&exception_class, JNIString::from(message))
}

fn throw_error(env: &mut jni::Env<'_>, caller_class: &JClass<'_>, error: NativeError) {
    let class_name = match &error {
        NativeError::Schema(_) => "SchemaException",
        NativeError::Open(_) => "IndexOpenException",
        NativeError::Write(_) => "WriteException",
        NativeError::Search(_) => "SearchException",
        NativeError::InvalidHandle(_) => "TantivyIndexClosedException",
        NativeError::Panic
        | NativeError::State(_)
        | NativeError::Json(_)
        | NativeError::Tantivy(_) => "NativeLibraryException",
    };
    let message = error.to_string();
    if throw_specific(env, caller_class, class_name, &message).is_err() {
        // The specific exception class may be unavailable (e.g. stripped or
        // renamed by R8 in a minified app). Never leave the error silent.
        let fallback: JNIString = "java/lang/RuntimeException".into();
        let fallback_message: JNIString =
            format!("{message} (failed to throw {EXCEPTION_PACKAGE}/{class_name})").into();
        let _ = env.throw_new(fallback, fallback_message);
    }
}

// Java string arguments must be read with an initialized Env (inside
// `with_env`); converting a JString before that yields "<JNI Not Initialized>"
// instead of the actual contents.
fn read_string(env: &jni::Env<'_>, value: &JString<'_>) -> NativeResult<String> {
    value
        .mutf8_chars(env)
        .map(|chars| chars.to_str().into_owned())
        .map_err(|error| NativeError::State(format!("failed to read Java string: {error}")))
}

fn jni_long<'caller>(
    mut env: EnvUnowned<'caller>,
    class: JClass<'caller>,
    action: impl FnOnce(&mut jni::Env<'caller>) -> NativeResult<i64>,
) -> jlong {
    env.with_env(|env| -> jni::errors::Result<jlong> {
        match run_native(env, action) {
            Ok(value) => Ok(value),
            Err(error) => {
                throw_error(env, &class, error);
                Ok(0)
            }
        }
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

fn jni_void<'caller>(
    mut env: EnvUnowned<'caller>,
    class: JClass<'caller>,
    action: impl FnOnce(&mut jni::Env<'caller>) -> NativeResult<()>,
) {
    env.with_env(|env| -> jni::errors::Result<()> {
        if let Err(error) = run_native(env, action) {
            throw_error(env, &class, error);
        }
        Ok(())
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

fn jni_string<'caller>(
    mut env: EnvUnowned<'caller>,
    class: JClass<'caller>,
    action: impl FnOnce(&mut jni::Env<'caller>) -> NativeResult<String>,
) -> jobject {
    env.with_env(|env| -> jni::errors::Result<jobject> {
        match run_native(env, action) {
            Ok(value) => Ok(JString::from_str(env, value)?.into_raw()),
            Err(error) => {
                throw_error(env, &class, error);
                Ok(std::ptr::null_mut())
            }
        }
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

macro_rules! export_handle_void {
    ($name:ident, $operation:ident) => {
        #[unsafe(no_mangle)]
        pub extern "system" fn $name<'caller>(
            env: EnvUnowned<'caller>,
            class: JClass<'caller>,
            handle: jlong,
        ) {
            jni_void(env, class, |_env| $operation(handle))
        }
    };
}

macro_rules! export_handle_string {
    ($name:ident, $operation:ident) => {
        #[unsafe(no_mangle)]
        pub extern "system" fn $name<'caller>(
            env: EnvUnowned<'caller>,
            class: JClass<'caller>,
            handle: jlong,
        ) -> jobject {
            jni_string(env, class, |_env| $operation(handle))
        }
    };
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rustedbytes_tantivy_NativeTantivy_nativeOpenIndex<'caller>(
    env: EnvUnowned<'caller>,
    class: JClass<'caller>,
    path: JString<'caller>,
    schema_json: JString<'caller>,
    options_json: JString<'caller>,
) -> jlong {
    jni_long(env, class, move |env| {
        let path = read_string(env, &path)?;
        let schema_json = read_string(env, &schema_json)?;
        let options_json = read_string(env, &options_json)?;
        open_index(&path, &schema_json, &options_json)
    })
}

export_handle_void!(
    Java_com_rustedbytes_tantivy_NativeTantivy_nativeCloseIndex,
    close_index
);

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rustedbytes_tantivy_NativeTantivy_nativeAddDocuments<'caller>(
    env: EnvUnowned<'caller>,
    class: JClass<'caller>,
    handle: jlong,
    documents_json: JString<'caller>,
) -> jobject {
    jni_string(env, class, move |env| {
        let documents_json = read_string(env, &documents_json)?;
        add_documents(handle, &documents_json)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rustedbytes_tantivy_NativeTantivy_nativeDeleteTerm<'caller>(
    env: EnvUnowned<'caller>,
    class: JClass<'caller>,
    handle: jlong,
    field: JString<'caller>,
    value_json: JString<'caller>,
) -> jobject {
    jni_string(env, class, move |env| {
        let field = read_string(env, &field)?;
        let value_json = read_string(env, &value_json)?;
        delete_term(handle, &field, &value_json)
    })
}

export_handle_string!(
    Java_com_rustedbytes_tantivy_NativeTantivy_nativeCommit,
    commit
);

export_handle_string!(
    Java_com_rustedbytes_tantivy_NativeTantivy_nativeRefresh,
    refresh
);

export_handle_string!(
    Java_com_rustedbytes_tantivy_NativeTantivy_nativeCommitAndRefresh,
    commit_and_refresh
);

export_handle_string!(
    Java_com_rustedbytes_tantivy_NativeTantivy_nativeSchemaInfo,
    schema_info
);

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rustedbytes_tantivy_NativeTantivy_nativeSearch<'caller>(
    env: EnvUnowned<'caller>,
    class: JClass<'caller>,
    handle: jlong,
    query_json: JString<'caller>,
) -> jobject {
    jni_string(env, class, move |env| {
        let query_json = read_string(env, &query_json)?;
        search(handle, &query_json)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rustedbytes_tantivy_NativeTantivy_nativeDeleteQuery<'caller>(
    env: EnvUnowned<'caller>,
    class: JClass<'caller>,
    handle: jlong,
    query: JString<'caller>,
    default_fields_json: JString<'caller>,
) -> jobject {
    jni_string(env, class, move |env| {
        let query = read_string(env, &query)?;
        let default_fields_json = read_string(env, &default_fields_json)?;
        delete_query(handle, &query, &default_fields_json)
    })
}

export_handle_string!(
    Java_com_rustedbytes_tantivy_NativeTantivy_nativeDeleteAllDocuments,
    delete_all_documents
);
