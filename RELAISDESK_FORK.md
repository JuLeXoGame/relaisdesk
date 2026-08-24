# Fork client RelaisDesk

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
source correspondant, y compris le protocole modifié et les instructions de
construction, conformément à l’AGPL v3.
