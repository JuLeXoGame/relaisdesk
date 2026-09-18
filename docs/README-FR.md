<p align="center">
  <img src="../res/relaisdesk-banner.png" alt="RelaisDesk - Solution souveraine de bureau à distance"><br>
  <a href="#étapes-brutes-de-la-compilationbuild">Build</a> -
  <a href="#comment-construire-avec-docker">Docker</a> -
  <a href="#structure-du-projet">Structure</a> -
  <a href="#captures-décran">Captures</a><br>
  [<a href="../README.md">English</a>] | [<a href="README-FR.md">Français</a>] | [<a href="README-ES.md">Español</a>] | [<a href="README-DE.md">Deutsch</a>] | [<a href="README-IT.md">Italiano</a>]<br>
  <b><a href="https://relaisdesk.fr">relaisdesk.fr</a> • Solution indépendante d'accès à distance et d'assistance informatique</b>
</p>

[![RelaisDesk](https://img.shields.io/badge/RelaisDesk-Site%20Officiel-blue)](https://relaisdesk.fr)
[![Licence AGPLv3](https://img.shields.io/badge/Licence-GNU%20AGPLv3-green)](../LICENCE)
[![Infrastructure France](https://img.shields.io/badge/Infrastructure-France%20(Oracle)-orange)](https://relaisdesk.fr)

Encore un autre logiciel de bureau à distance, écrit en Rust. Fonctionne directement, aucune configuration n'est nécessaire. Vous avez le contrôle total de vos données, sans aucun souci de sécurité. Vous pouvez utiliser notre serveur de rendez-vous/relais, [configurer le vôtre](https://rustdesk.com/server), ou [écrire votre propre serveur de rendez-vous/relais](https://github.com/rustdesk/rustdesk-server-demo).

RustDesk accueille les contributions de tout le monde. Voir [`docs/CONTRIBUTING.md`](CONTRIBUTING.md) pour plus d'informations.

[**TÉLÉCHARGEMENT BINAIRE**](https://github.com/rustdesk/rustdesk/releases)

## Dépendances

Les versions de bureau utilisent [sciter](https://sciter.com/) pour l'interface graphique, veuillez télécharger la bibliothèque dynamique sciter vous-même.

[Windows](https://raw.githubusercontent.com/c-smile/sciter-sdk/master/bin.win/x64/sciter.dll) |
[Linux](https://raw.githubusercontent.com/c-smile/sciter-sdk/master/bin.lnx/x64/libsciter-gtk.so)
[macOS](https://raw.githubusercontent.com/c-smile/sciter-sdk/master/bin.osx/libsciter.dylib)

## Étapes brutes de la compilation/build

- Préparez votre environnement de développement Rust et votre environnement de compilation C++.

- Installez [vcpkg](https://github.com/microsoft/vcpkg), et définissez correctement la variable d'environnement `VCPKG_ROOT`.

  - Windows : vcpkg install libvpx:x64-windows-static libyuv:x64-windows-static opus:x64-windows-static aom:x64-windows-static
  - Linux/macOS : vcpkg install libvpx libyuv opus aom

- Exécutez `cargo run`

## Comment compiler/build sous Linux

### Ubuntu 18 (Debian 10)

```sh
sudo apt install -y g++ gcc git curl wget nasm yasm libgtk-3-dev clang libxcb-randr0-dev libxdo-dev libxfixes-dev libxcb-shape0-dev libxcb-xfixes0-dev libasound2-dev libpulse-dev cmake
```

### Fedora 28 (CentOS 8)

```sh
sudo yum -y install gcc-c++ git curl wget nasm yasm gcc gtk3-devel clang libxcb-devel libxdo-devel libXfixes-devel pulseaudio-libs-devel cmake alsa-lib-devel
```

### Arch (Manjaro)

```sh
sudo pacman -Syu --needed unzip git cmake gcc curl wget yasm nasm zip make pkg-config clang gtk3 xdotool libxcb libxfixes alsa-lib pipewire
```

### Installer vcpkg

```sh
git clone https://github.com/microsoft/vcpkg
cd vcpkg
git checkout 2023.04.15
cd ..
vcpkg/bootstrap-vcpkg.sh
export VCPKG_ROOT=$HOME/vcpkg
vcpkg/vcpkg install libvpx libyuv opus aom
```

### Corriger libvpx (Pour Fedora)

```sh
cd vcpkg/buildtrees/libvpx/src
cd *
./configure
sed -i 's/CFLAGS+=-I/CFLAGS+=-fPIC -I/g' Makefile
sed -i 's/CXXFLAGS+=-I/CXXFLAGS+=-fPIC -I/g' Makefile
make
cp libvpx.a $HOME/vcpkg/installed/x64-linux/lib/
cd
```

### Construire

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
git clone https://github.com/rustdesk/rustdesk
cd rustdesk
mkdir -p target/debug
wget https://raw.githubusercontent.com/c-smile/sciter-sdk/master/bin.lnx/x64/libsciter-gtk.so
mv libsciter-gtk.so target/debug
cargo run
```

## Comment construire avec Docker

Commencez par cloner le dépôt et construire le conteneur Docker :

```sh
git clone https://github.com/rustdesk/rustdesk
cd rustdesk
docker build -t "rustdesk-builder" .
```

Ensuite, chaque fois que vous devez compiler le logiciel, exécutez la commande suivante :

```sh
docker run --rm -it -v $PWD:/home/user/rustdesk -v rustdesk-git-cache:/home/user/.cargo/git -v rustdesk-registry-cache:/home/user/.cargo/registry -e PUID="$(id -u)" -e PGID="$(id -g)" rustdesk-builder
```

Notez que la première compilation peut prendre plus de temps avant que les dépendances ne soient mises en cache, les compilations suivantes seront plus rapides. De plus, si vous devez spécifier différents arguments à la commande de compilation, vous pouvez le faire à la fin de la commande à la position `<OPTIONAL-ARGS>`. Par exemple, si vous voulez compiler une version de release optimisée, vous devez exécuter la commande ci-dessus suivie de `--release`. L'exécutable résultant sera disponible dans le dossier cible sur votre système, et peut être lancé avec :

```sh
target/debug/rustdesk
```

Ou, si vous exécutez un exécutable provenant d'une release :

```sh
target/release/rustdesk
```

Veuillez vous assurer que vous exécutez ces commandes à partir de la racine du dépôt RustDesk, sinon l'application ne pourra pas trouver les ressources requises. Notez également que les autres sous-commandes de cargo telles que `install` ou `run` ne sont pas actuellement supportées par cette méthode car elles installeraient ou exécuteraient le programme à l'intérieur du conteneur au lieu de l'hôte.

## Structure du projet

- **[libs/hbb_common](https://github.com/rustdesk/rustdesk/tree/master/libs/hbb_common)** : codec vidéo, config, wrapper tcp/udp, protobuf, fonctions fs pour le transfert de fichiers, et quelques autres fonctions utilitaires.
- **[libs/scrap](https://github.com/rustdesk/rustdesk/tree/master/libs/scrap)** : capture d'écran
- **[libs/enigo](https://github.com/rustdesk/rustdesk/tree/master/libs/enigo)** : contrôle clavier/souris spécifique à la plate-forme
- **[src/ui](https://github.com/rustdesk/rustdesk/tree/master/src/ui)** : interface graphique
- **[src/server](https://github.com/rustdesk/rustdesk/tree/master/src/server)** : services audio/clipboard/input/vidéo, et connexions réseau
- **[src/client.rs](https://github.com/rustdesk/rustdesk/tree/master/src/client.rs)** : démarrer une connexion entre pairs
- **[src/rendezvous_mediator.rs](https://github.com/rustdesk/rustdesk/tree/master/src/rendezvous_mediator.rs)** : Communiquer avec [rustdesk-server](https://github.com/rustdesk/rustdesk-server), attendre une connexion distante directe (TCP hole punching) ou relayée.
- **[src/platform](https://github.com/rustdesk/rustdesk/tree/master/src/platform)** : code spécifique à la plateforme

> [!Attention]
> **Avertissement contre l'utilisation abusive:** <br>
> Les développeurs de RustDesk ne cautionnent ni ne soutiennent aucune utilisation non éthique ou illégale de ce logiciel. Toute utilisation abusive, telle que l'accès non autorisé, le contrôle ou l'invasion de la vie privée, est strictement contraire à nos directives. Les auteurs ne sont pas responsables de toute utilisation abusive de l'application.

## Captures d'écran

### Espace Technicien RelaisDesk (Gestion de parc & accès distant)
![Espace Technicien RelaisDesk](screenshots/relaisdesk-espace-technicien.png)

### Session distante en direct
![Session distante RelaisDesk](screenshots/relaisdesk-session-distante.png)

