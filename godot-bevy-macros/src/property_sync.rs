use crate::bevy_attr::{ClassPlan, Mapping, SyncMode, component_mappings};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Ident, Member, Path, Type};

struct Field<'a> {
    mapping: &'a Mapping,
    component: usize,
    member: Member,
    godot_type: Type,
}

pub struct PropertySync<'a> {
    state: Ident,
    register: Ident,
    components: Vec<&'a Path>,
    fields: Vec<Field<'a>>,
}

impl<'a> PropertySync<'a> {
    pub fn new(plan: &'a ClassPlan, input: &DeriveInput) -> Option<Self> {
        let mut components = Vec::new();
        let mut fields = Vec::new();
        let mut add = |path, mappings: &'a [Mapping]| {
            let component = components.len();
            components.push(path);
            for mapping in mappings.iter().filter(|mapping| mapping.sync.is_some()) {
                let member = mapping
                    .bevy_field
                    .clone()
                    .map(Member::Named)
                    .unwrap_or_else(|| {
                        Member::Unnamed(syn::Index::from(mapping.tuple_index.unwrap_or(0)))
                    });
                let godot_type = mapping.as_type.clone().unwrap_or_else(|| {
                    let Data::Struct(data) = &input.data else {
                        unreachable!()
                    };
                    let field = if let Some(index) = mapping.tuple_index {
                        data.fields.iter().nth(index)
                    } else {
                        data.fields
                            .iter()
                            .find(|f| f.ident.as_ref() == Some(&mapping.godot_prop))
                    };
                    field.expect("live mapping source field").ty.clone()
                });
                fields.push(Field {
                    mapping,
                    component,
                    member,
                    godot_type,
                });
            }
        };
        if !plan.primary.path.segments.is_empty() {
            add(&plan.primary.path, &plan.primary.fields);
        }
        for component in &plan.companions {
            add(&component.path, component_mappings(&component.init));
        }
        if fields.is_empty() {
            return None;
        }
        Some(Self {
            state: format_ident!("__{}PropertySync", plan.godot_class),
            register: format_ident!(
                "__register_{}_property_sync",
                plan.godot_class.to_string().to_lowercase()
            ),
            components,
            fields,
        })
    }

    pub fn registration(&self) -> TokenStream {
        let register = &self.register;
        quote!(Some(#register))
    }

    fn validation(&self) -> Option<TokenStream> {
        if self.components.len() < 2 {
            return None;
        }
        let components = &self.components;
        let class = self.state.to_string();
        Some(quote!(godot_bevy::plugins::property_sync::valid_components(
            &[#((::std::any::TypeId::of::<#components>(), ::std::any::type_name::<#components>())),*],
            #class,
        )))
    }

    pub fn seed(&self, values: &[TokenStream]) -> TokenStream {
        let state = &self.state;
        let validation = self
            .validation()
            .map(|check| quote!(if !#check { return false; }));
        let names: Vec<_> = (0..values.len())
            .map(|i| format_ident!("component_{i}"))
            .collect();
        let initial = self.fields.iter().enumerate().map(|(i, field)| {
            let component = &names[field.component];
            let member = &field.member;
            let b = format_ident!("bevy_{i}");
            let p = format_ident!("pending_{i}");
            let g = format_ident!("godot_{i}");
            let gty = &field.godot_type;
            let godot = (field.mapping.sync == Some(SyncMode::TwoWay)).then(|| quote!(
                #g: godot_bevy::plugins::property_sync::empty_snapshot(
                    |value: &#gty| (&&godot_bevy::plugins::property_sync::Value(value)).capture()
                ),
            ));
            quote!(#b: #component.#member.clone(), #p: false, #godot)
        });
        quote! {
            use godot_bevy::plugins::property_sync::Capture as _;
            #validation
            #(let #names = #values;)*
            let state = #state {
                instance_id: handle.instance_id(),
                #(#initial)*
            };
            commands.entity(entity).insert((#(#names,)* state));
        }
    }

    pub fn emit(&self, class: &Ident) -> TokenStream {
        let state = &self.state;
        let register = &self.register;
        let types: Vec<_> = (0..self.fields.len())
            .map(|i| format_ident!("B{i}"))
            .collect();
        let validation = self
            .validation()
            .map(|check| quote!(if !#check { return; }));
        let mut all_types = types.clone();
        let mut value_checks = Vec::new();
        let mut state_fields = Vec::new();
        let mut parameters = Vec::new();
        let mut arguments = Vec::new();
        let mut reads = Vec::new();
        let mut samples = Vec::new();
        let mut sample_names = Vec::new();
        let mut outputs = Vec::new();
        let mut writes = Vec::new();
        let mut output_names = Vec::new();
        let mut read_queries = Vec::new();
        let mut write_queries = Vec::new();
        let mut query_names = Vec::new();
        let mut reconciles = Vec::new();

        for (component_index, component) in self.components.iter().enumerate() {
            let fields: Vec<_> = self
                .fields
                .iter()
                .enumerate()
                .filter(|(_, f)| f.component == component_index)
                .collect();
            if fields.is_empty() {
                continue;
            }
            let name = format_ident!("component_{component_index}");
            let mutable = fields
                .iter()
                .any(|(_, f)| f.mapping.sync == Some(SyncMode::TwoWay));
            read_queries.push(if mutable {
                quote!(Option<&mut #component>)
            } else {
                quote!(Option<godot_bevy::bevy_ecs::change_detection::Ref<#component>>)
            });
            write_queries
                .push(quote!(Option<godot_bevy::bevy_ecs::change_detection::Ref<#component>>));
            query_names.push(name.clone());
            let reset = fields.iter().map(|(i, field)| {
                let b = format_ident!("bevy_{i}");
                let p = format_ident!("pending_{i}");
                let g = format_ident!("godot_{i}");
                let get = format_ident!("get_{i}");
                let godot = (field.mapping.sync == Some(SyncMode::TwoWay))
                    .then(|| quote!(state.#g = None;));
                quote! {
                    state.#b = #get(component).clone();
                    state.#p = true;
                    #godot
                }
            });
            reconciles.push(quote! {
                if let Some(component) = &#name {
                    if rebound || (component.is_added() && component.added().is_newer_than(state_tick, ticks.this_run())) {
                        #(#reset)*
                    }
                }
            });
        }

        for (i, field) in self.fields.iter().enumerate() {
            let bty = &types[i];
            let gty = &field.godot_type;
            let component = self.components[field.component];
            let component_name = format_ident!("component_{}", field.component);
            let member = &field.member;
            let prop = &field.mapping.godot_prop;
            let b = format_ident!("bevy_{i}");
            let g = format_ident!("godot_{i}");
            let p = format_ident!("pending_{i}");
            let get = format_ident!("get_{i}");
            let set = format_ident!("set_{i}");
            let from = format_ident!("from_{i}");
            let into = format_ident!("into_{i}");
            let output = format_ident!("output_{i}");
            output_names.push(output.clone());
            state_fields.push(quote!(#b: #bty, #p: bool,));
            parameters.push(quote!(#get: fn(&#component) -> &#bty));
            arguments.push(quote!(|component| &component.#member));
            parameters.push(quote!(#into: fn(#bty) -> #gty));
            arguments.push(
                field
                    .mapping
                    .into
                    .as_ref()
                    .map_or_else(|| quote!(|value| value), |f| quote!(#f)),
            );
            let two_way = field.mapping.sync == Some(SyncMode::TwoWay);
            let capture = format_ident!("capture_{i}");
            value_checks.push(quote! {
                let _ = |value: &#gty| (&&godot_bevy::plugins::property_sync::Value(value)).capture();
            });
            if two_way {
                let snapshot_ty = format_ident!("S{i}");
                all_types.push(snapshot_ty.clone());
                parameters.push(quote!(#capture: fn(&#gty) -> #snapshot_ty));
                arguments.push(quote!(|value: &#gty| (&&godot_bevy::plugins::property_sync::Value(value)).capture()));
                state_fields.push(quote!(#g: Option<#snapshot_ty>,));
                parameters.push(quote!(#set: fn(&mut #component) -> &mut #bty));
                arguments.push(quote!(|component| &mut component.#member));
                parameters.push(quote!(#from: fn(#gty) -> #bty));
                arguments.push(
                    field
                        .mapping
                        .with
                        .as_ref()
                        .map_or_else(|| quote!(|value| value), |f| quote!(#f)),
                );
                let sample_name = format_ident!("sample_{i}");
                sample_names.push(sample_name.clone());
                samples.push(quote! {
                    #component_name.as_ref().and_then(|component| {
                        if state.#p || !equal(#get(component), &state.#b) {
                            state.#p = true;
                            None
                        } else {
                            stats.read();
                            let sample = #capture(&node.#prop);
                            if state.#g.as_ref().is_none_or(|previous| !equal(&sample, previous)) {
                                Some((node.#prop.clone(), sample))
                            } else { None }
                        }
                    })
                });
                reads.push(quote! {
                    if let (Some(component), Some((godot_value, sample))) = (&mut #component_name, #sample_name) {
                        let candidate = #from(godot_value);
                        if !equal(#get(component), &candidate) {
                            *#set(component) = candidate.clone();
                        }
                        state.#b = candidate;
                        state.#g = Some(sample);
                    }
                });
            }
            outputs.push(quote! {
                let #output = #component_name.as_ref().and_then(|component| {
                    let value = #get(component);
                    if state.#p || !equal(value, &state.#b) {
                        state.#p = true;
                        Some((#into(value.clone()), value.clone()))
                    } else {
                        None
                    }
                });
            });
            let godot = two_way.then(|| quote!(let godot_snapshot = #capture(&output);));
            let accept_godot = two_way.then(|| quote!(state.#g = Some(godot_snapshot);));
            writes.push(quote! {
                if let Some((output, bevy)) = #output {
                    #godot
                    node.#prop = output;
                    #accept_godot
                    state.#b = bevy;
                    state.#p = false;
                    stats.write();
                }
            });
        }
        let read_node = (!reads.is_empty()).then(|| {
            quote! {
                let Some(node) = godot.try_get::<#class>(*handle) else { continue; };
                if node.is_queued_for_deletion() { continue; }
                let (#(#sample_names,)*) = {
                    let node = node.bind();
                    (#(#samples,)*)
                };
                #(#reads)*
            }
        });
        let reconcile = quote! {
            let state_tick = state.last_changed();
            let rebound = state.instance_id != handle.instance_id();
            #(#reconciles)*
            if rebound { state.instance_id = handle.instance_id(); }
        };
        quote! {
            #[derive(godot_bevy::bevy_ecs::component::Component)]
            struct #state<#(#all_types: Send + Sync + 'static),*> {
                instance_id: godot::obj::InstanceId,
                #(#state_fields)*
            }

            fn #register(app: &mut godot_bevy::bevy_app::App) {
                use godot_bevy::plugins::property_sync::Capture as _;
                #validation
                #(#value_checks)*
                #[allow(clippy::too_many_arguments, clippy::type_complexity, unused_mut, unused_variables)]
                fn register<#(#all_types: Clone + PartialEq + Send + Sync + 'static),*>(
                    app: &mut godot_bevy::bevy_app::App,
                    #(#parameters,)*
                ) {
                    use godot_bevy::bevy_ecs::change_detection::DetectChanges;
                    use godot_bevy::bevy_ecs::schedule::IntoScheduleConfigs;
                    use godot_bevy::plugins::property_sync::{PropertySyncSet, equal, PropertySyncStats};
                    godot_bevy::plugins::property_sync::init_stats(app);
                    app.add_systems(godot_bevy::bevy_app::PreUpdate,
                        (move |mut godot: godot_bevy::interop::GodotAccess,
                               mut query: godot_bevy::bevy_ecs::system::Query<(
                                   &godot_bevy::interop::GodotNodeHandle,
                                   &mut #state<#(#all_types),*>,
                                   #(#read_queries,)*
                               )>,
                               ticks: godot_bevy::bevy_ecs::system::SystemChangeTick,
                               mut stats: PropertySyncStats| {
                            stats.read_pass();
                            for (handle, mut state, #(mut #query_names,)*) in &mut query {
                                #reconcile
                                #read_node
                            }
                        }).in_set(PropertySyncSet::Read)
                    );
                    app.add_systems(godot_bevy::bevy_app::PostUpdate,
                        (move |mut godot: godot_bevy::interop::GodotAccess,
                               mut query: godot_bevy::bevy_ecs::system::Query<(
                                   &godot_bevy::interop::GodotNodeHandle,
                                   &mut #state<#(#all_types),*>,
                                   #(#write_queries,)*
                               )>,
                               ticks: godot_bevy::bevy_ecs::system::SystemChangeTick,
                               mut stats: PropertySyncStats| {
                            stats.write_pass();
                            for (handle, mut state, #(#query_names,)*) in &mut query {
                                #reconcile
                                #(#outputs)*
                                if !(false #(|| #output_names.is_some())*) { continue; }
                                let Some(mut node) = godot.try_get::<#class>(*handle) else { continue; };
                                if node.is_queued_for_deletion() { continue; }
                                let mut node = node.bind_mut();
                                #(#writes)*
                            }
                        }).in_set(PropertySyncSet::Write)
                    );
                }
                register(app, #(#arguments),*);
            }
        }
    }
}
