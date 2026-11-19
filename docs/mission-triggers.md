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

## Interpretador Rust

O executável lê os scripts diretamente do contêiner para o censo e para o
teste com entradas sintéticas. Rode a partir da pasta que contém o
`RESOURCE.RES`:

```sh
cargo run --release -- --check-missions
cargo run --release -- --trace-mission c1m3.mis 'time, 3' 'intact, P'
```

O censo em Rust leu 46 arquivos e 2.150 regras. O segundo comando trata
somente os predicados fornecidos como verdadeiros e registra as ações
emitidas. Com as entradas acima, saem três ações no primeiro tick e
nenhuma no segundo. O teste mede o interpretador, sem áudio, IA ou estado
de voo. No protótipo jogável, o mesmo interpretador recebe o estado do voo para
conduzir uma missão de treinamento com três pontos sequenciais.

O avaliador provisório trata `and` e `andn` como parte de uma conjunção
e usa `or` para começar a próxima conjunção. A repetição das mesmas
restrições em ramos separados de `c3m2` (linhas 465–493) sustenta essa
hipótese. O DOS ainda precisa confirmar a precedência, a ordem dos
efeitos e se a execução única se aplica a todos os gatilhos.

| Arquivo | Regras | Uso na validação |
|---|---:|---|
| `c1m3.mis` | 53 | Primeiro fluxo de missão e três ocorrências de `andn` |
| `c3m4.mis` | 50 | Dez ações `setai` sem predicados novos em relação a `c1m3`. O argumento com NUL na linha 241 é preservado pelo parser |
| `c3m2.mis` | 43 | Dois casos de `or` com `andn` na mesma regra. Caso prioritário para distinguir precedência |
| `c1m3`, `c1m1`, `c4m4`, `c5m7` | 240 | Cobertura dos 34 nomes distintos de condições, ações e conectivos da campanha |

Os quatro arquivos da última linha reduzem o volume a examinar para
cobrir a sintaxe. A validação de comportamento também precisa de casos
que exercitem argumentos, limiares e ordens de eventos diferentes.
