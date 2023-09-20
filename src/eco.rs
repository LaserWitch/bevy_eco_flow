//! in the bottom right. For text within a scene, please see the text2d example.

use bevy::{
    prelude::*, utils::HashMap, diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},};

#[derive(Component)]
struct ReadoutText;

//use crate::{FpsText, ColorText};
pub struct EcoPlugin;
impl Plugin for EcoPlugin{
    fn build(&self, app: &mut App) {
        app.insert_resource(ResourceMap::default())
            .add_systems(Startup, setup_eco)
            .add_systems(Update, (
                run_producers
                ,apply_caps
                ,run_converters
                ,(output,output_gui)
            ).chain());
        ()
    }
}
pub fn setup_eco(mut commands: Commands,mut res:ResMut<ResourceMap>, asset_server: Res<AssetServer>){
    let cooling = commands.spawn((
        Label("Cooling".to_string()),
        Storage(100.),
        StorageLimit(100.)
    )).id();

    res.insert("Cooling".to_string(),cooling);
    commands.spawn((
        Produce(vec![(cooling,0.1)]),
        Stack(1.),
        Label("Radiators".to_string())));

    let energy = commands.spawn((
            Label("Energy".to_string()),
            Storage(100.),
            StorageLimit(200.)
        )).id();
    res.insert("Energy".to_string(),energy);

    commands.spawn((
        Produce(vec![(energy,10.)]),
        Consume(vec![(cooling,10.2)]),
        Stack(1.),
        Label("Generators".to_string())));

    for s in vec!["Mass"]{
        let e = commands.spawn((
            Label(s.to_string().clone()),
            Storage(0.),
            )).id();
        res.insert(s.to_string(), e);
    }

    // UI camera
    commands.spawn(Camera2dBundle::default());
    // Text with one section
    commands.spawn((
        // Create a TextBundle that has a Text with a single section.
        TextBundle::from_section(
            // Accepts a `String` or any type that converts into a `String`, such as `&str`
            "hello\nbevy!",
            TextStyle {
                // This font is loaded and will be used instead of the default font.
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 100.0,
                color: Color::WHITE,
            },
        ) // Set the alignment of the Text
        .with_text_alignment(TextAlignment::Left)
        // Set the style of the TextBundle itself.
        .with_style(Style {
            position_type: PositionType::Relative,
            bottom: Val::Vh(00.0),
            right: Val::Vw(00.0),
            ..default()
        }),
        ReadoutText,
    ));
    // Text with multiple sections
    commands.spawn((
        // Create a TextBundle that has a Text with a list of sections.
        TextBundle::from_sections([
            TextSection::new("hello\nbevy!", TextStyle {
                // This font is loaded and will be used instead of the default font.
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 100.0,
                color: Color::WHITE,
            },),
            TextSection::default(),
            TextSection::default(),
        ])
        .with_text_alignment(TextAlignment::Left)
        // Set the style of the TextBundle itself.
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(15.0),
            ..default()
        }),
    ));

    
}

type  TypedQuantity =(Entity,f32);

#[derive(Component,Default,Deref,DerefMut)]
pub struct Label(pub String);

#[derive(Resource,Default,Deref,DerefMut)]
pub struct ResourceMap(pub HashMap<String,Entity>);

#[derive(Component,Default,Deref,DerefMut)]
pub struct Storage(pub f32);

#[derive(Component,Default,Deref,DerefMut)]
pub struct StorageLimit(pub f32);

#[derive(Component,Default)]
pub struct AllowOverflow;


#[derive(Component,Default,Deref,DerefMut)]
pub struct Stack(
    pub f32);

#[derive(Component,Default)]
pub struct Produce(Vec<TypedQuantity>);

#[derive(Component,Default)]
pub struct Consume(Vec<TypedQuantity>);

//Process is:
//  Producers generates resources, uncapped
//  Converters 
//      count total demands
//      generates production
//  Sinks apply
//  storage caps apply
//  strings are updated
//  button events are handled.
fn run_producers(
    mut storage:Query<(Entity, &mut Storage)>,
    producers:Query<(&Produce,Option<&Stack>),Without<Consume>>,
    time : Res<Time>,
    ){
    let dt = time.delta_seconds();
    let mut sums :HashMap<Entity,f32> = default();
    for (p,s) in &producers{
        let stack = f32::floor( if s.is_some() {s.unwrap().0} else {1. });
        for (res, rate) in p.0.iter(){
            let amount = rate*dt*stack;
            match sums.get_mut(res){
                Some(v) => {
                    *v=*v+amount; },
                _ => {sums.insert(*res, amount);}
            }
        }
    }
    for (e, mut s) in &mut storage{
        let p_opt = sums.get(&e);
        let p = if p_opt.is_some()  {*(p_opt.unwrap())} else {0.0} ;
        s.0 = (s.0) + p;
    }

}
fn run_converters(
    mut storage:Query<(Entity, &mut Storage, Option<&StorageLimit>, Option<&Stack>)>,
    converters:Query<(&Produce,&Consume,Option<&Stack>)>,
    time : Res<Time>){
    let dt = time.delta_seconds();
    for (p,c,s) in &converters{
        let storage = &mut storage;
        let stack = f32::floor( if s.is_some() {s.unwrap().0} else {1. });
        let mut satisfaction:f32 = 1.0;
        //find demand satisfaction
        for (store_e,rate) in &c.0{
            let amount = rate*dt*stack;
            let store = storage.get_component_mut::<Storage>(*store_e).unwrap();
            if store.0 <= 0. {
                satisfaction= 0.;}
            else{
                satisfaction=satisfaction.min(store.0/amount);
            }
        }
        //don't run over cap 
        for (store_e,_) in &p.0{
            let (_,storage,opt_limit,opt_stack) = storage.get(*store_e).unwrap();
            if opt_limit.is_some(){
                let limit = opt_limit.unwrap().0*
                    if opt_stack.is_none() {1.} else {opt_stack.unwrap().0};
                if storage.0 >= limit{
                    satisfaction = 0.;
                }
            }
        }
        //spend actual money
        for (store_e,rate) in &c.0{
            let amount = rate*dt*stack*satisfaction;
            let mut store = storage.get_component_mut::<Storage>(*store_e).unwrap();
            store.0 = f32::max(store.0-amount,0.0);
        }
        //get paid
        for (store_e,rate) in &p.0{
            let amount = rate*dt*stack*satisfaction;
            let mut store = storage.get_component_mut::<Storage>(*store_e).unwrap();
            store.0 = store.0+amount;
        }
    }

}
fn _run_sinks(
    mut _storage:Query<(Entity, &mut Storage)>,
    _conv:Query<(&Produce,&Consume,Option<&Stack>)>,
    _time : Res<Time>){

}
fn apply_caps(mut storage:Query<(&mut Storage,&StorageLimit,Option<&Stack>)>){
    for (mut store,limit,stack) in &mut storage{
        let stack = f32::floor( if stack.is_some() {stack.unwrap().0} else {1. });
        let limit = (limit.0) * stack;
        store.0 = f32::min(limit,store.0);
    }
}

fn output( val:Query<(Entity,&Storage,Option<&Label>)>,
        mut i:Local<u32> ){
    if *i > 150 {
        *i=0;
    }
    if *i==0{
        info!("");
        for (e,s,l) in &val{
            if l.is_some(){
                info!(" {} : {}",l.expect("").0,s.0);
            }
            else{
                
                info!(" {:?} : {}",e,s.0);
            }
        }
    }
    *i+=1;

}
fn output_gui(
    val:Query<(Entity,&Storage,Option<&Label>)>,
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<ReadoutText>>,
) {
    info!("gui");
    let mut eco = "".to_string();
    for (e,s,l) in &val{
        if l.is_some(){
            eco=format!("{}{} : {:.2}\n",eco,l.expect("").0,s.0).to_string();
        }
        else{
            eco=format!("{}{:?} : {:.2}\n",eco,e,s.0).to_string();
        }
        info!("eco");
    }
    for ref mut text in &mut query {

        info!("before: {:?}", text);
        text.sections[0].value = eco.clone();
        info!("after: {:?}", text);
    }
    info!(eco);
}