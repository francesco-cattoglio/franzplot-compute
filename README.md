# franzplot-compute

## Stato attuale

Feature già implementate:
- save e load del grafo da file, incluse le variabili globali usate
- undo/redo nel grafo. Oltre ad avere i due tasti Undo/Redo, è possibile usare gli shortcut CTRL+Z e CTRL+SHIFT+Z
- duplicazione dei nodi
- controllo della telecamera in stile VTK. È possibile aggiungere altri stili (es: fly camera o orbit come nel vecchio franzplot)
- possibile selezionare gli oggetti facendoci doppioclick nella scena
- curve di bezier
- zoom nel grafo

Known bugs & issues:
- in Debug l'avvio è piuttosto lento; in Release l'avvio è molto più veloce (5 secondi vs 0.5).

Feature previste per Gennaio:
- rendering & trasformazioni di primitive
- raggruppamento dei nodi
- visualizzazione delle curve u-v sulle superfici
- scelta del formato file definitivo (attualmente è json, ma non può contenere commenti quindi va cambiato. Idealmente dovrebbe avere un versioning)

Features a data da destinarsi:
- dump del grafo in caso di crash
- settings (tipo di telecamera, visualizzazione degli assi cartesiani, visualizzazioni di piani, eccetera
- trasparenza, via screen door transparency o qualsiasi altra tecnica purché sia Order Indipendent
- widget a-la Blender per visualizzazione degli assi e possibile switch a prospettiva ortografica
- import/export di mesh
- export di png
- sample della curva (la visualizzazione delle famiglie u-v è più utile didatticamente!)
- export di video

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
