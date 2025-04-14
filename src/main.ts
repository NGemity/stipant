import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event"

let data_dir: HTMLInputElement | null;
let export_dir: HTMLInputElement | null;

let filename: HTMLInputElement | null;

let filehash: HTMLInputElement | null;
let offset: HTMLInputElement | null;
let size: HTMLInputElement | null;
let location: HTMLInputElement | null;

let dump_file: HTMLButtonElement | null;
let dump_all: HTMLButtonElement | null;
let filecount: HTMLParagraphElement | null;

type RZFile = {
  hash: string;
  name: string;
  offset: number;
  size: number;
  file: number;
}

async function select_data_dir() {
  await invoke("select_data_dir");
}

async function select_export_dir() {
  console.log("select export dir");
  await invoke("select_export_dir");
}

async function get_filename(name: string) {
  await invoke("get_filename", { filename: name })
}

window.addEventListener("DOMContentLoaded", () => {
  // Set dirs
  data_dir = document.querySelector("#data_dir");
  export_dir = document.querySelector("#export_dir");
  // Set dynamic input
  filename = document.querySelector("#filename");
  // Set app-written inputs
  filehash = document.querySelector("#filehash");
  offset = document.querySelector("#offset");
  size = document.querySelector("#size");
  location = document.querySelector("#location");

  // Set buttons
  dump_file = document.querySelector("#dump_file_btn");
  dump_all = document.querySelector("#dump_all_btn");
  filecount = document.querySelector("#filecount");

  document.querySelector("#select-data-dir")?.addEventListener("submit", (e) => {
    e.preventDefault();
    select_data_dir();
  });
  document.querySelector("#select-export-dir")?.addEventListener("submit", (e) => {
    e.preventDefault();
    select_export_dir();
  });

  if(filename) {
    filename.addEventListener("input", () => {
      get_filename(filename?.value!);
    });
  }
});

listen<string>('set_data_dir', (event) => {
  if (event.payload.length <= 0) {
    return;
  }
  if(!data_dir || !filename || !dump_file || !dump_all || !filecount) {
    return;
  }
  data_dir.value = event.payload;

  filename.disabled = false;
  dump_file.disabled = false;
  dump_all.disabled = false;
});

listen<string>('set_export_dir', (event) => {
  if (event.payload.length <= 0) {
    return;
  }
  if(!export_dir) {
    return;
  }
  export_dir.value = event.payload;
});

listen<number>('set_filecount', (event) => {
  if(!filecount) {
    return;
  }
  filecount.innerText = event.payload.toString();
});

listen<RZFile>('set_data', (event) => {
  if(!filehash || !size || !offset || !location) 
    return;

  filehash.value = event.payload.hash;
  size.value = event.payload.size.toString();
  offset.value = event.payload.offset.toString();
  location.value = `data.00${event.payload.file.toString()}`
});