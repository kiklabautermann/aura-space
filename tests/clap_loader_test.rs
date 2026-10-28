//! Integration test loading the compiled CLAP dynamic library.

use clap_sys::factory::plugin_factory;
use std::ffi::CStr;

#[test]
fn test_clap_library_entry() {
    // Path to the compiled release library
    let lib_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/libaura_verb.so");

    if !lib_path.exists() {
        eprintln!("Release library not built yet, skipping dynamic load test.");
        return;
    }

    unsafe {
        let lib = libloading::Library::new(&lib_path)
            .expect("Failed to dynamically load compiled CLAP plugin");

        let entry_symbol: libloading::Symbol<*const clap_sys::entry::clap_plugin_entry> = lib
            .get(b"clap_entry")
            .expect("Failed to locate clap_entry symbol in .so");

        let entry = &**entry_symbol;
        assert!(entry.init.is_some(), "clap_entry.init must be present");
        assert!(
            entry.get_factory.is_some(),
            "clap_entry.get_factory must be present"
        );

        // Call init
        let path_c = std::ffi::CString::new(lib_path.to_str().unwrap()).unwrap();
        let init_ok = (entry.init.unwrap())(path_c.as_ptr());
        assert!(init_ok, "clap_entry.init returned false");

        // Query plugin factory
        let factory_fn = entry.get_factory.unwrap();
        let factory_ptr = factory_fn(plugin_factory::CLAP_PLUGIN_FACTORY_ID.as_ptr() as *const _);
        assert!(!factory_ptr.is_null(), "Factory ptr should not be null");

        let factory = factory_ptr as *const plugin_factory::clap_plugin_factory;
        let get_descriptor = (*factory).get_plugin_descriptor.expect("get_plugin_descriptor");
        let desc_ptr = get_descriptor(factory, 0);
        assert!(!desc_ptr.is_null(), "Descriptor ptr should not be null");

        let desc = &*desc_ptr;
        let id = CStr::from_ptr(desc.id).to_str().unwrap();
        let name = CStr::from_ptr(desc.name).to_str().unwrap();
        let vendor = CStr::from_ptr(desc.vendor).to_str().unwrap();

        println!("Successfully loaded CLAP plugin:");
        println!("  ID:     {}", id);
        println!("  Name:   {}", name);
        println!("  Vendor: {}", vendor);

        assert_eq!(id, "com.aura.vintageverb");
        assert_eq!(name, "AuraVintageVerb");
        assert_eq!(vendor, "Aura Bridge");
    }
}
