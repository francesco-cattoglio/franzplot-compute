# franzplot-compute

## Stato attuale

Tutte le feature necessarie per un uso di base del software sono state implementate.

Feature per la release per gli studenti:
- [x] Cleanup dell'interfaccia
- [] Settings per la sensività dello zoom su grafo e scena
- [] Settings per la telecamera - orbit sempre verticale?
- [x] Aggiunta delle primitive mancanti
- [x] Rimozione dei materiali non strettamente necessari
- [] Update all'ultima versione di imnodes per i bugfix
- [] Etichettare gli assi
- [] Dimensione del piano?

Known bugs & issues:
- in Debug l'avvio è piuttosto lento; in Release l'avvio è molto più veloce (5 secondi vs 0.5).

Features a data da destinarsi:
- implementazione della visualizzazione delle curve u-v sulle superfici
- raggruppamento dei nodi
- dump del grafo in caso di crash
- settings (tipo di telecamera, visualizzazione degli assi cartesiani, visualizzazioni di piani, eccetera
- widget a-la Blender per visualizzazione degli assi e possibile switch a prospettiva ortografica
- import/export di mesh
- export di png
- export di video

## Come compilare il progetto

Le tre dipendenze per compilare il contenuto del repository sotto linux sono:
- la toolchain per il linguaggio Rust, *versione minima 1.48*
- le librerie per lo sviluppo della API grafica vulkan
- un compilatore C++11 e CMake (necessari per le dipendenze)

### Toolchain Rust
Il consiglio è quello di seguire le istruzioni che trovate sul sito ufficiale: https://www.rust-lang.org/tools/install

### Librerie Vulkan
Queste le potete installare direttamente dai repository della vostra distribuzione. I pacchetti sono:
- Per Ubuntu: `libvulkan-dev` e `vulkan-tools`
- Per Arch Linux: `spirv-tools` e il driver vulkan per la vostra scheda video, ad esempio `vulkan-intel` o `vulkan-radeon`

### Compilazione
Una volta che avete i prerequisiti, dovrebbe essere sufficiente dare il comando `cargo run` all'interno della cartella principale del repository per scaricare tutte le altre dipendenze, compilarle in automatico e infine compilare il franzplot e lanciarlo. L'eseguibile viene creato nella sottocartella `target`; come alternativa se volete solo compilare basta il comando `cargo build`.
Per compilare in release, `cargo build --release` o `cargo run --release`.
