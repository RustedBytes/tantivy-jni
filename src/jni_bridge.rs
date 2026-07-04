use std::panic::{AssertUnwindSafe, catch_unwind};

use jni::EnvUnowned;
use jni::objects::{JClass, JString};
use jni::strings::JNIString;
use jni::sys::{jlong, jobject};

use crate::{
    NativeError, NativeResult, add_documents, close_index, commit, commit_and_refresh,
    delete_all_documents, delete_query, delete_term, open_index, refresh, schema_info, search,
};

const EXCEPTION_PACKAGE: &str = "com/rustedbytes/tantivy";

fn run_native<T>(action: impl FnOnce() -> NativeResult<T>) -> NativeResult<T> {
    catch_unwind(AssertUnwindSafe(action)).map_err(|_| NativeError::Panic)?
}

fn throw_error(env: &mut jni::Env<'_>, error: NativeError) {
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
    let class: JNIString = format!("{EXCEPTION_PACKAGE}/{class_name}").into();
    let message = error.to_string();
    if env
        .throw_new(class, JNIString::from(message.as_str()))
        .is_err()
    {
        // The specific exception class may be unavailable (e.g. stripped or
        // renamed by R8 in a minified app). Never leave the error silent.
        let fallback: JNIString = "java/lang/RuntimeException".into();
        let fallback_message: JNIString =
            format!("{message} (failed to throw {EXCEPTION_PACKAGE}/{class_name})").into();
        let _ = env.throw_new(fallback, fallback_message);
    }
}

fn read_string(value: JString<'_>) -> String {
    value.to_string()
}

fn jni_long<'caller>(
    mut env: EnvUnowned<'caller>,
    action: impl FnOnce() -> NativeResult<i64>,
) -> jlong {
    env.with_env(|env| -> jni::errors::Result<jlong> {
        match run_native(action) {
            Ok(value) => Ok(value),
            Err(error) => {
                throw_error(env, error);
                Ok(0)
            }
        }
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

fn jni_void<'caller>(mut env: EnvUnowned<'caller>, action: impl FnOnce() -> NativeResult<()>) {
    env.with_env(|env| -> jni::errors::Result<()> {
        if let Err(error) = run_native(action) {
            throw_error(env, error);
        }
        Ok(())
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

fn jni_string<'caller>(
    mut env: EnvUnowned<'caller>,
    action: impl FnOnce() -> NativeResult<String>,
) -> jobject {
    env.with_env(|env| -> jni::errors::Result<jobject> {
        match run_native(action) {
            Ok(value) => Ok(JString::from_str(env, value)?.into_raw()),
            Err(error) => {
                throw_error(env, error);
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
            _class: JClass<'caller>,
            handle: jlong,
        ) {
            jni_void(env, || $operation(handle))
        }
    };
}

macro_rules! export_handle_string {
    ($name:ident, $operation:ident) => {
        #[unsafe(no_mangle)]
        pub extern "system" fn $name<'caller>(
            env: EnvUnowned<'caller>,
            _class: JClass<'caller>,
            handle: jlong,
        ) -> jobject {
            jni_string(env, || $operation(handle))
        }
    };
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rustedbytes_tantivy_NativeTantivy_nativeOpenIndex<'caller>(
    env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    path: JString<'caller>,
    schema_json: JString<'caller>,
    options_json: JString<'caller>,
) -> jlong {
    let path = read_string(path);
    let schema_json = read_string(schema_json);
    let options_json = read_string(options_json);
    jni_long(env, || open_index(&path, &schema_json, &options_json))
}

export_handle_void!(
    Java_com_rustedbytes_tantivy_NativeTantivy_nativeCloseIndex,
    close_index
);

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rustedbytes_tantivy_NativeTantivy_nativeAddDocuments<'caller>(
    env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    handle: jlong,
    documents_json: JString<'caller>,
) -> jobject {
    let documents_json = read_string(documents_json);
    jni_string(env, || add_documents(handle, &documents_json))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rustedbytes_tantivy_NativeTantivy_nativeDeleteTerm<'caller>(
    env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    handle: jlong,
    field: JString<'caller>,
    value_json: JString<'caller>,
) -> jobject {
    let field = read_string(field);
    let value_json = read_string(value_json);
    jni_string(env, || delete_term(handle, &field, &value_json))
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
    _class: JClass<'caller>,
    handle: jlong,
    query_json: JString<'caller>,
) -> jobject {
    let query_json = read_string(query_json);
    jni_string(env, || search(handle, &query_json))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_rustedbytes_tantivy_NativeTantivy_nativeDeleteQuery<'caller>(
    env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    handle: jlong,
    query: JString<'caller>,
    default_fields_json: JString<'caller>,
) -> jobject {
    let query = read_string(query);
    let default_fields_json = read_string(default_fields_json);
    jni_string(env, || delete_query(handle, &query, &default_fields_json))
}

export_handle_string!(
    Java_com_rustedbytes_tantivy_NativeTantivy_nativeDeleteAllDocuments,
    delete_all_documents
);
