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
