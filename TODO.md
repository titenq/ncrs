# TODO

- [x] Tornar os caminhos de configuração TLS portáteis.
  - Substituir o uso manual de `HOME/.config/ncrs` por um crate cross-platform.
  - Crate preferido: `directories`.
  - Comportamento esperado:
    - Linux: respeitar `XDG_CONFIG_HOME`; se não estiver definido, usar `~/.config/ncrs`.
    - Windows: usar o diretório de configuração apropriado em AppData.
    - macOS: usar `~/Library/Application Support`.
  - Armazenar os arquivos TLS como:
    - `config_dir()/cert.pem`
    - `config_dir()/key.pem`
  - Manter fallback local para `./cert.pem` e `./key.pem` apenas se ainda quisermos facilitar testes dentro do projeto.

- [x] Tornar a geração de certificados TLS independente do OpenSSL externo.
  - O `--tls-gen` atual usa o comando `openssl`.
  - Substituir por uma implementação em Rust puro antes da distribuição para Windows.
  - Objetivo: `ncrs.exe --tls-gen` deve funcionar sem exigir `openssl.exe` no `PATH`.

- [x] Implementar a distribuição para Windows, macOS e Linux.
  - [x] Configurar CI/CD com GitHub Actions (`.github/workflows/release.yml`).
  - [x] Gerar executáveis nativos via `windows-latest`, `ubuntu-latest` e `macos-latest`.
  - [x] Automatizar o anexo de binários na página de Releases do GitHub quando uma nova tag `v*` é publicada.
  - [x] Validar caminhos de configuração, geração TLS, TCP, UDP e comportamento de scan no Windows.
