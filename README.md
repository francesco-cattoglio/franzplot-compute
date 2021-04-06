# franzplot-compute

## Stato attuale

Tutte le feature necessarie per un uso di base del software sono state implementate.

Known bugs & issues:
- in Debug l'avvio è piuttosto lento; in Release l'avvio è molto più veloce (5 secondi vs 0.5).
- la Translation Matrix è stata implementata con un metodo quick&dirt, qualora si decidesse di
  aggiungere la possibilità di operare sui vettori in qualsiasi maniera (ad es, applicargli trasformazioni
  o calcolarli come sottrazione di due punti), sarà necessario re-implementarla usando un nuovo tipo di
  compute block.

Features a data da destinarsi:
- implementazione della visualizzazione delle curve u-v sulle superfici
- raggruppamento dei nodi
- dump del grafo in caso di crash
- widget a-la Blender per visualizzazione degli assi e possibile switch a prospettiva ortografica
- import/export di mesh
- export di video

## Come compilare il progetto
Sono indispensabili su tutte le piattaforme:
- la toolchain per il linguaggio Rust, *versione minima 1.48*
- un compilatore C++11 e CMake (necessari per le dipendenze). Per CMake sotto MacOS va bene la versione installabile via homebrew.

Sotto windows, è necessario avere:
- il compilatore C++ msvc. Mingw64 ha dato problemi in passato, quindi si consiglia di evitarlo.
- il build tool ninja (https://github.com/ninja-build/ninja/releases), l'eseguibile deve trovarsi in una cartella che faccia parte del PATH.

Sotto Linux:
- le librerie per lo sviluppo della API grafica vulkan
- le librerie di sviluppo per gtk3

Sotto MacOS:
- sono necessari i tool per compilare da riga di comando; se a build compare l'errore `xcrun: error: invalid active developer path`, dare il comando `xcode-select --install`

### Toolchain Rust
Il consiglio è quello di seguire le istruzioni che trovate sul sito ufficiale: https://www.rust-lang.org/tools/install

### Librerie Vulkan
Queste le potete installare direttamente dai repository della vostra distribuzione. I pacchetti sono:
- Per Ubuntu: `libvulkan-dev` e `vulkan-tools`
- Per Arch Linux: `spirv-tools` e il driver vulkan per la vostra scheda video, ad esempio `vulkan-intel` o `vulkan-radeon`

### Compilazione
Una volta che avete i prerequisiti, dovrebbe essere sufficiente dare il comando `cargo run` all'interno della cartella principale del repository per scaricare tutte le altre dipendenze, compilarle in automatico e infine compilare il franzplot e lanciarlo. L'eseguibile viene creato nella sottocartella `target`; come alternativa se volete solo compilare basta il comando `cargo build`.

Per compilare in release, `cargo build --release` o `cargo run --release`.

Per compilare con la feature che consente di visualizzare i timestamp dei file aperti, dare il comando `cargo build --features "show-timestamps"`
