// Pas de console sous Windows en version finale (sans effet sur Mac).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    insitu_app_lib::run()
}
