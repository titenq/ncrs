# TODO

- OpenBSD options such as `-F`, `-M`, `-m`, `-P`, `-r`, `-S`, `-T`, `-t`, `-W`, `-X`, `-x`, and `-Z` are not implemented yet.

-r (Random Ports): Sorteia portas de origem ou de destino aleatoriamente, em vez de sequencialmente. Excelente para obfuscar Port Scans (-z).

-W <limite> (Limite de Recebimento): Encerra o programa após receber exatamente X pacotes/linhas de resposta. Muito útil para scripts rápidos.

-x e -X (Proxy Support): Faz o ncrs se conectar através de proxies SOCKS4/SOCKS5 ou HTTP. É super poderoso, mas dá um pouco mais de trabalho por causa do handshake do proxy.
