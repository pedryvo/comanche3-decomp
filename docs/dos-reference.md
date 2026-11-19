# Referência de execução no DOS

A cópia local chegou ao cockpit de treinamento em Fort Rucker no DOSBox
0.74-3. Foi possível abrir o menu em voo com Esc. O teste usou uma cópia
dos arquivos em um diretório temporário.

A execução sem placa de som parou em **Downloading Mission Parameters**.
A execução que avançou usou esta configuração:

```ini
[dosbox]
memsize=64
[cpu]
core=dynamic
cycles=max
[mixer]
nosound=false
[sblaster]
sbtype=sb16
sbbase=220
irq=5
dma=1
hdma=5
```

No Linux, o teste de vídeo usou `SDL_VIDEODRIVER=x11` e o áudio do host
usou `SDL_AUDIODRIVER=dummy`. Isso mantém a placa emulada disponível ao
jogo e direciona a saída de áudio a um dispositivo temporário sem som.

A abertura do cockpit estabelece uma referência para medir os gatilhos.
A ordem dos eventos e a precedência das condições precisam de
experimentos em execução. A matriz está no [censo de gatilhos](mission-triggers.md).

## Relógio no menu de voo

Duas capturas separadas por cinco segundos mostraram o menu de Esc aberto
com o relógio em 00:00:20 e 00:00:25. O texto do tutorial também mudou entre
as capturas. Para medir o tempo de um gatilho, abrir esse menu não serve
para interromper a simulação. A tecla Pause exibiu a mensagem de pausa,
mas a duração da interrupção ainda não foi isolada.

## Áudio e vídeo da instalação

`--check-audio` decodifica `INSIDE.WAV` e `ALERT.WAV` diretamente do
`RESOURCE.RES`. O primeiro reproduz o som contínuo do motor na missão
de treinamento. O segundo sinaliza a confirmação de um objetivo.

`--check-videos` inspeciona os arquivos KDV locais. `HINTRO.KDV`,
`LINTRO.KDV` e `LOVRVIEW.KDV` começam em 320 × 240, com
registros com identificador de quatro bytes e tamanho little-endian
que inclui o próprio cabeçalho do registro. O utilitário verifica
os limites dos registros até o fim do arquivo. Em `LINTRO.KDV`, o
terceiro segmento muda para 320 × 180. Os codecs e a
temporização ainda precisam ser identificados antes da reprodução.
