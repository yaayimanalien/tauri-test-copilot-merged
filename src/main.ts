import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { initAppearance } from "$lib/appearance.svelte";

initAppearance();
mount(App, { target: document.getElementById("app")! });
