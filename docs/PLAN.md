# Plano de trabalho

## Fase 0 — Base do MVP

O MVP em Rust já abre em Linux, lê o terreno do `RESOURCE.RES` fornecido pelo jogador e renderiza o voo em 1080p e 4K. O parser e o interpretador de gatilhos já funcionam pelo terminal. O parser processou 46 arquivos e 2.150 regras. A ordem de avaliação e os efeitos observados nos scripts ainda aguardam comparação com o DOS. [MVP e execução](../README.md) · [Censo e rastreamento de gatilhos](mission-triggers.md)

## Fase 1 — Primeira missão jogável

O primeiro percurso implementado é um treinamento de navegação com três pontos de passagem sobre o terreno do MVP. O teste automatizado cobre a sequência, a passagem no solo ou fora de ordem e o reinício. A verificação manual do percurso ainda está pendente. O interpretador consulta a posição e o estado de voo do jogador, avança o objetivo na ordem do percurso e encerra a missão após o terceiro ponto.

| Frente | Trabalho | Dependência |
|---|---|---|
| Voo | Expor o estado de decolagem e a posição ao interpretador | Simulação de voo do MVP |
| Gatilhos | Definir o percurso e ligar condições e ações ao estado da missão | Parser e interpretador; estado de voo |
| Interface | Mostrar objetivo ativo, progresso, conclusão e reinício | Estado da missão |
| Verificação | Exercitar o percurso, a passagem fora de ordem e o reinício | Integração entre voo, gatilhos e interface |

**Critério de conclusão.** O jogador consegue decolar, cruzar os três pontos em ordem e receber a mensagem de conclusão. Aproximar-se do ponto no solo ou cruzar um ponto fora de ordem mantém o objetivo atual. Reiniciar volta ao primeiro ponto e permite concluir o percurso novamente. Executar `--demo` para observar a sessão na janela e `--smoke` para testar o percurso sem abrir uma janela.

## Fase 2 — Verificação de compatibilidade

Usar a matriz existente como amostra para comparar a interpretação dos dados com observações registradas no DOS. `c1m3`, `c1m1`, `c4m4` e `c5m7` cobrem os 34 nomes distintos em 240 regras. Usar `c3m2` para investigar a precedência quando `or` e `andn` aparecem juntos, e `c3m4` para investigar os efeitos de `setai` e o argumento com byte NUL. [Matriz e casos prioritários](mission-triggers.md#interpretador-rust)

**Critério de conclusão.** Registrar entradas, sequência de ações e divergências para cada caso testado. Documentar quais interpretações têm confirmação observável e quais continuam como hipóteses. A cobertura dos nomes mede a sintaxe. Limiares, argumentos e efeitos exigem seus próprios casos.

## Fase 3 — Expansão do protótipo

A partir do percurso jogável e das verificações de compatibilidade, ampliar o conjunto de condições e ações de uso geral e criar novas missões de treinamento. Validar a execução em Windows, testar teclado e câmera em ambas as plataformas e medir o desempenho em 1080p e 4K.

**Critério de conclusão.** Executar os mesmos testes de percurso e reinício em Linux e Windows, registrar resultados por resolução e demonstrar que uma nova missão pode reutilizar o interpretador e o estado de voo.
