# franzplot-compute

## Stato attuale

Feature già implementate:
- save e load del grafo da file, incluse le variabili globali usate.
- undo/redo nel grafo. Oltre ad avere i due tasti Undo/Redo, è possibile usare gli shortcut CTRL+Z e CTRL+SHIFT+Z.
- duplicazione dei nodi.
- controllo della telecamera in stile VTK. È possibile aggiungere altri stili (es: fly camera o orbit come nel vecchio franzplot) 

Known bugs:
- Quando si seleziona un nodo nel grafo con click sinistro e poi si fa click destro su un nodo diverso, le azioni vengono eseguite sul nodo selezionato, non sull'ultimo che è stato clickato.
- Curve perfettamente verticali (segmento parallelo all'asse z) non vengono renderizzate correttamente.

Prossime feature:
- rendering dei punti
- miglior rendering delle curve e delle superfici
- gestione della scena (in particolare, le luci e gli assi cartesiani)
- export di png

Feature previste per Gennaio:
- rendering & trasformazioni di primitive
- import/export di mesh
- curve di bezier
- raggruppamento dei nodi nel grafo
- visualizzazione delle curve u-v sulle superfici
- scelta del formato file definitivo (attualmente uso un json, ma non può contenere commenti quindi va cambiato)

Features non indispensabili per ora:
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
