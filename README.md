# Investigação do Comanche 3 para Rust

MVP de voo em Rust com o terreno de `c1m3` lido diretamente do `RESOURCE.RES` da edição DOS. O objetivo do protótipo é decolar e cruzar o waypoint mostrado no HUD.

## Jogar

Com Rust estável instalado, execute na pasta que contém o `RESOURCE.RES` original:

```sh
cargo run --release -- --1080p
```

Use Espaço para subir, Shift para descer, W/S para acelerar e frear, A/D para virar, R para reiniciar e Esc para fechar. Segure Espaço e W para decolar na direção inicial do waypoint.

Use `--4k` para renderizar a 3840×2160, `--demo` para reproduzir o voo automaticamente na janela e `--smoke` para verificar carregamento, renderização e conclusão do waypoint sem janela. A janela é redimensionável. O MVP usa renderização por CPU e foi validado no Linux.

O código deste repositório usa [GPL-3.0-only](LICENSE). Os dados do jogo são fornecidos pelo jogador e ficam fora do repositório.

## Censo de gatilhos

O contêiner analisado foi o `RESOURCE.RES` local, SHA-256
`f86d208f967ca65eec9efb5f86c1c4defa21018d2daa3cc815f15b76022f679b`.
A extração foi feita em diretório temporário com o `pup` do [Sioux](https://github.com/hanatos/sioux/tree/c5cadf87540d041991c76091a7897ab881bac5ed/pup), revisão
`c5cadf87540d041991c76091a7897ab881bac5ed`. Para repetir, a partir deste repositório e com o `pup` já compilado:

```sh
mission_data=$(mktemp -d)
/caminho/para/pup --plugin res --unpack "$PWD/RESOURCE.RES" "$mission_data/"
python3 tools/trigger_census.py "$mission_data"
```

- Os 41 arquivos nomeados como campanha têm 2.124 regras, 21 condições, 10 ações e três conectivos (`and`, `or`, `andn`). Cada regra tem uma ação e até oito condições. Quarenta arquivos têm `win`. O bloco de gatilhos de `c1m6` está vazio. A presença de um arquivo no pacote não confirma que ele aparece no menu.
- Os cinco arquivos nomeados `mp` têm 26 regras, com `time`, `text` e `end`. O censo cobre os scripts e não mede a implementação de rede.
- `c1m3` tem 53 regras, 14 tipos de condição e cinco ações. Sua regra de vitória combina quatro testes `destroyed` e um `intact` para o jogador. Dos 40 arquivos de campanha com regras, apenas `c1m3` e `c2m9` usam exclusivamente esse vocabulário.
- `play`, `text`, `route` e `clearcounter` respondem por 1.891 das 2.124 ações. Essa frequência ajuda a ordenar o trabalho, mas uma ação rara ainda pode impedir o progresso de uma missão.

## Próximas verificações

Um interpretador comum pode carregar todos esses blocos. O próximo teste é comparar a regra de vitória de `c1m3` com uma execução no DOS e, em seguida, usar uma missão com `andn` e outra com `setai` para verificar a combinação de condições e a fronteira entre o script e a IA. A ordem da avaliação, o momento em que um gatilho dispara e as unidades de distância e tempo ainda precisam de confirmação em execução. A configuração do Ghidra no MCP do REA bloqueou a análise de funções do executável DOS MZ, portanto este censo usa os dados da distribuição e mede cobertura de sintaxe.
