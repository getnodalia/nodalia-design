# Ilustración del icono de una app

`portrait.py` convierte un retrato histórico en la ilustración del icono de una app Nodalia: busca
la figura en la imagen, la recorta del fondo, la pasa a blanco y negro con algo más de contraste y
la pone, con una sombra suave detrás, sobre un campo a sangre del color de la app. Sale un PNG de
1024 px que cada app convierte en su SVG maestro y en todos sus tamaños (la regla y la lista de
figuras están en `docs/iconography.md`).

## Uso

```sh
pip install "rembg[cpu]" pillow   # el modelo de recorte se descarga la primera vez
python3 tools/app-portrait/portrait.py RETRATO SALIDA.png --box L,T,R,B --colors ARRIBA ABAJO
```

- `--box`: el cuadrado del retrato, en píxeles del original, que llena la baldosa. Puede salirse
  de la imagen (lo que queda fuera es fondo). Encuadra cabeza y hombros, con la cabeza ocupando
  más o menos la mitad del ancho, para que se lea a 32 px.
- `--colors`: los dos tonos del campo, arriba y abajo, los mismos del icono anterior y de la
  ventana del DMG de la app.
- `--gamma`: por encima de 1 oscurece los medios tonos, útil en fotos quemadas o grabados claros.
- `--add-mask X,Y,X,Y,...`: un polígono, en píxeles del original, que se suma a la figura cuando
  el modelo pierde un hombro en un fondo del mismo tono.
- `--model`: el modelo de rembg; `u2net_human_seg` (el de por defecto) es el que mejor separa
  retratos.

## Ejemplos

```sh
# Trama: Jacquard, del retrato tejido en seda de 1839 (Wikimedia Commons, dominio público).
python3 tools/app-portrait/portrait.py "A_la_mémoire_de_J.M._Jacquard.jpg" trama.png \
  --box 430,322,810,702 --colors '#9A45E2' '#7A26C9' --gamma 1.35

# Ánima: Émile Reynaud (Wikimedia Commons, dominio público).
python3 tools/app-portrait/portrait.py "Émile_Reynaud.png" anima.png --box 0,-30,470,440 \
  --colors '#6B4CE8' '#5034C6' --gamma 1.25 \
  --add-mask 300,300,340,300,400,330,440,380,470,420,470,572,300,572
```

Mira siempre el resultado a 512, 64 y 16 px antes de llevarlo a la app.
