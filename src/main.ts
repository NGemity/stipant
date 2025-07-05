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
  file: number;
  found: boolean;
  size: number;
  offset: number;
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

async function dump_filename(name: string) {
  await invoke("dump_filename", { filename: name })
}

async function dump_all_rust() {
  await invoke("dump_all");
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
  document.querySelector("#dump_file")?.addEventListener("submit", (e) => {
    if (!filename)
      return;
    e.preventDefault();
    dump_filename(filename.value);
  });

  document.querySelector("#dump_all")?.addEventListener("submit", (e) => {
    e.preventDefault();
    dump_all_rust();
  });

  if (filename) {
    filename.addEventListener("input", () => {
      get_filename(filename?.value!);
    });
  }
});



listen<string>('set_data_dir', (event) => {
  if (event.payload.length <= 0) {
    return;
  }
  if (!data_dir || !filename || !filecount) {
    return;
  }
  data_dir.value = event.payload;

  filename.disabled = false;
});

listen<string>('set_export_dir', (event) => {
  if (event.payload.length <= 0) {
    return;
  }
  if (!export_dir || !dump_file || !dump_all) {
    return;
  }
  export_dir.value = event.payload;
  dump_file.disabled = false;
  dump_all.disabled = false;
});

listen<number>('set_filecount', (event) => {
  if (!filecount) {
    return;
  }
  filecount.innerText = event.payload.toString();
});

listen<RZFile>('set_data', (event) => {
  if (!filehash || !size || !offset || !location)
    return;

  console.log(event.payload);
  if(event.payload.found === true) {
    filehash.value = event.payload.hash;
    size.value = event.payload.size.toString();
    offset.value = event.payload.size.toString();
    location.value = `data.00${event.payload.file.toString()}`;
  } else {
    console.log("Not found?")
    filehash.value = "";
    size.value = "";
    offset.value = "";
    location.value = "";
  }
});