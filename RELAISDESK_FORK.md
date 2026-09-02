# Fork client RelaisDesk — avis de modification

Modifications réalisées par **Julien BELLOT, entrepreneur individuel (EI),
Informatique A Domicile 03 / RelaisDesk**, SIREN 940 747 108. Première date
pertinente des modifications RelaisDesk : **24 août 2026**.

Ce dépôt dérive du client RustDesk Community, révision amont
`1d09760ef7c9275555ac512d66fee5af549c5d06`. Il reste sous GNU AGPL v3 ; voir
`LICENCE` et les avis de tiers du projet.

Le fork ajoute le protocole d’autorisation RelaisDesk : lecture à chaud d’un
jeton court depuis un fichier privé, signature de preuve avec une clé locale,
authentification des enregistrements `hbbs`, des demandes P2P, des demandes
`hbbr` et d’un canal de contrôle périodique sur les deux extrémités. En cas
d’expiration ou de refus du technicien ou du viewer, le client concerné ferme
la session au lieu de continuer avec une configuration copiée.

Les options supplémentaires de `RustDesk2.toml` sont :

```toml
relaisdesk-token-file = '/chemin/prive/network-token'
relaisdesk-proof-key-file = '/chemin/prive/proof-key'
```

Ces fichiers ne doivent jamais être intégrés dans un installateur, journalisés
ou placés dans le dépôt. Le configurateur les crée localement et renouvelle le
jeton via l’API.

Toute personne recevant le binaire doit pouvoir obtenir gratuitement le code
source correspondant à ce binaire, y compris le sous-module `libs/hbb_common`
modifié, le protocole ajouté, les scripts et les instructions de construction,
conformément à l’AGPL v3. Le dépôt publié doit conserver les avis de copyright
d'origine et identifier le commit exact ayant servi au build.

Le dossier `libs/pam-safe` dérive de `rustdesk-org/pam` au commit
`7bfd25510202cd269292cbdd7c71f3977a6fd762`. Il conserve les licences MIT et
Apache-2.0 d'origine. RelaisDesk y remplace la dépendance Unix `users`, non
maintenue, et évite de modifier l'environnement global du processus depuis le
thread d'authentification.

RustDesk et ses marques appartiennent à leurs titulaires respectifs. Ce fork
est indépendant et n'est ni affilié à RustDesk ni approuvé par ses titulaires,
sauf accord écrit contraire.
