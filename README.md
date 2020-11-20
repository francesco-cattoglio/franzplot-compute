# franzplot-compute

## Stato attuale

La struttura base dell'interfaccia è pronta. È possibile aggiungere e rimuovere nodi dal grafo,
impostare variabili globali, renderizzare la scena. Le principali feature da implementare sono:
- save e load del grafo da file
- vari fix, in particolare uno riguardo il rendering delle curve
- movimento della telecamera

Sviluppi futuri:
- trovare un modo per catturare il mouse che funzioni senza dare problemi
- undo/redo nel grafo
- copiaincolla dei nodi
- gestione della scena: luci e assi cartesiani

## Come compilare il progetto

Le tre dipendenze per compilare il contenuto del repository sotto linux sono:
- la toolchain per il linguaggio Rust, *versione minima 1.48*
- le librerie per lo sviluppo della API grafica vulkan
- un compilatore C++11

### Toolchain Rust
Il consiglio è quello di seguire le istruzioni che trovate sul sito ufficiale: https://www.rust-lang.org/tools/install

### Librerie Vulkan
Queste le potete installare direttamente dai repository della vostra distribuzione. I pacchetti sono:
- Per Ubuntu: `libvulkan-dev` e `vulkan-tools`
- Per Arch Linux: `spirv-tools` e il driver vulkan per la vostra scheda video, ad esempio `vulkan-intel` o `vulkan-radeon`

### Compilazione
Una volta che avete i prerequisiti, dovrebbe essere sufficiente dare il comando `cargo run` all'interno della cartella principale del repository per scaricare tutte le altre dipendenze, compilarle in automatico e infine compilare il franzplot e lanciarlo. L'eseguibile viene creato nella sottocartella `target`; come alternativa se volete solo compilare basta il comando `cargo build`.
Per compilare in release, `cargo build --release` o `cargo run --release`.
