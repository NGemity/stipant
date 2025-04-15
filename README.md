<p align="center">
        <img height="100px" src="/src/assets/ngemity.png" />
</p>

<h2 align="center">stipant<br>Unpacker for the ROSE Framekwork</h2>

### Intro
In search for a small project to play around with [tauri](https://tauri.app) and Rust I decided to rewrite a tool we've using for years by Pyrok, his Data Unpacker. 

#### Dump All
Dump all has been improved to use a single thread per data-file.  
On a packed archive with around 100k files I can achieve sub 2 second for dumping every file.

### Use your own keys
If you want to use your own keys, you have two possible options:
#### Use environment variables
```dosini
STIPANT_RESOURCE_ENCODE_KEY=YOUR_KEY_IN_BASE64
STIPANT_DEC_TABLE=YOUR_KEY_IN_BASE64
STIPANT_REF_TABLE=YOUR_KEY_IN_BASE64
STIPANT_DECRYPTED_EXTENSIONS="jpg;png;cob"
```  
#### Use config file
Create a `config.yaml` file and place it in the folder of the running directory (should be the folder of the stipant executable).  
```yaml
resource_encode_key: "your_key_in_base64" # Has to be in Base64
dec_table: "your_key_in_base64" # Has to be in Base64
ref_table: "your_key_in_base64" # Has to be in Base64
decrypted_extensions: "jpg;png;cob" # multiple extensions, splittable by ;
```

#### Build it yourself
Due to dependencies on WebView, I can only release packed installers (see Github Action).  
If you want to build the executable yourself to not rely on an installer, do the following:

- [Install Rust](https://www.rust-lang.org/tools/install)
- [Install tauri](https://v2.tauri.app/):
- [Install PNPM](https://pnpm.io/installation)
- Build stipent
  ```ps
    cd src-tauri/
    cargo tauri build
   ```


## Screenshot

<p align="center">
        <img src="/.github/screenshot.png" />
</p>1