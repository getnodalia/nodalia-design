# Icon candidates (2026-11-18)

Candidate app-icon portraits: black-and-white historical portrait on the app colour, 1024 px, clipped to the rounded tile (radius 112/512 = 224 px at 1024, transparent corners). One `<app>-sheet.png` per app (candidate at 360 px, plus 64 px and 16 px); the individual 1024 px PNGs sit beside them.

Field colours are the approximations given in the brief (top / bottom); the DMG colours in `tools/dmg-background/draw_background.py` differ for Forja, Hito, Pliego and Vinilo.

Cut-out: BiRefNet-portrait matte on the crop that fills the tile, main figure only, smooth contour pulled in ~0.5 px (no halo), edge colour decontamination, soft side fade where shoulders cannot fit. Scripts in `pipeline/`. `scale` = tile px per source px (below 1 = downsampled, never upscaled).

## Forja

| Candidate | Figure | Source (Commons) | Author | Licence | Source px | Scale | Note |
|---|---|---|---|---|---|---|---|
| `forja-eiffel.png` | Gustave Eiffel | [Gustave_Eiffel_1888_Nadar2.jpg...](https://commons.wikimedia.org/wiki/File:Gustave_Eiffel_1888_Nadar2.jpg) | Nadar | Public domain | 2128x3152 | 0.79 |  |
| `forja-brunel.png` | Isambard Kingdom Brunel | [1857_Isambard_Kingdom_Brunel_with_the_la...](https://commons.wikimedia.org/wiki/File:1857_Isambard_Kingdom_Brunel_with_the_launching_chains_of_the_steamship_Great_Eastern_by_Robert_Howlett_(restoration1)_(cropped).jpg) | Robert Howlett | Public domain | 2373x2371 | 0.56 | hat brim ~0.5 of tile width |
| `forja-tesla.png` | Nikola Tesla | [Tesla_Sarony.jpg...](https://commons.wikimedia.org/wiki/File:Tesla_Sarony.jpg) | Napoleon Sarony | Public domain | 3702x4986 | 0.26 |  |

## Trama

| Candidate | Figure | Source (Commons) | Author | Licence | Source px | Scale | Note |
|---|---|---|---|---|---|---|---|
| `trama-morris.png` | William Morris | [William_Morris_age_53.jpg...](https://commons.wikimedia.org/wiki/File:William_Morris_age_53.jpg) | Frederick Hollyer | Public domain | 1404x1767 | 0.68 | coat continues past the source edge: soft side fade at right |
| `trama-lovelace.png` | Ada Lovelace | [Ada_Lovelace_portrait.jpg...](https://commons.wikimedia.org/wiki/File:Ada_Lovelace_portrait.jpg) | Alfred Edward Chalon | Public domain | 2439x3504 | 0.47 | watercolour converted to B/W |
| `trama-babbage.png` | Charles Babbage | [Charles_Babbage_-_1860.jpg...](https://commons.wikimedia.org/wiki/File:Charles_Babbage_-_1860.jpg) | Unknown author | Public domain | 2862x4478 | 0.48 |  |
| `trama-jacquard.png` | Joseph Marie Jacquard | [Lyon_2e_-_Archives_municipales_de_Lyon_-...](https://commons.wikimedia.org/wiki/File:Lyon_2e_-_Archives_municipales_de_Lyon_-_Exposition_Au_service_de_la_ville_-_Portrait_de_Joseph_Marie_Jacquard,_par_Jean-Claude_Bonnefond.jpeg) | Romainbehar | CC0 | 2322x4128 | 0.46 | photo of the painting; frame cropped out; soft fade at right where the coat meets the canvas edge |

## Vinilo

| Candidate | Figure | Source (Commons) | Author | Licence | Source px | Scale | Note |
|---|---|---|---|---|---|---|---|
| `vinilo-berliner.png` | Emile Berliner | [Emile_Berliner_LCCN2014717588.tif...](https://commons.wikimedia.org/wiki/File:Emile_Berliner_LCCN2014717588.tif) | Bain News Service, publisher | Public domain | 3715x5009 | 0.29 |  |
| `vinilo-caruso.png` | Enrico Caruso | [Mr._Enrico_Caruso_(tenor)_-_DPLA_-_98cd5...](https://commons.wikimedia.org/wiki/File:Mr._Enrico_Caruso_(tenor)_-_DPLA_-_98cd5d213377f832189a49f03fd88bae_(page_1).jpg) | Mishkin Studio, New York (photo; Commons lists book author P | Public domain | 2513x4101 | 0.37 |  |
| `vinilo-beethoven.png` | Ludwig van Beethoven | [Joseph_Karl_Stieler%27s_Beethoven_mit_de...](https://commons.wikimedia.org/wiki/File:Joseph_Karl_Stieler%27s_Beethoven_mit_dem_Manuskript_der_Missa_solemnis.jpg) | Joseph Karl Stieler | Public domain | 4330x5389 | 0.28 | wide torso: soft side fade left/right |
| `vinilo-edison.png` | Thomas Edison | [Thomas_Edison2.jpg...](https://commons.wikimedia.org/wiki/File:Thomas_Edison2.jpg) | Louis Bachrach, Bachrach Studios, restored by Michel Vuijlst | Public domain | 2888x3696 | 0.64 | wide torso: soft side fade |

## Pliego

| Candidate | Figure | Source (Commons) | Author | Licence | Source px | Scale | Note |
|---|---|---|---|---|---|---|---|
| `pliego-franklin.png` | Benjamin Franklin | [Joseph_Siffrein_Duplessis_-_Benjamin_Fra...](https://commons.wikimedia.org/wiki/File:Joseph_Siffrein_Duplessis_-_Benjamin_Franklin_-_Google_Art_Project.jpg) | Joseph-Siffred Duplessis | Public domain | 3249x4000 | 0.34 |  |
| `pliego-baskerville.png` | John Baskerville | [John_Baskerville_by_James_Millar.jpg...](https://commons.wikimedia.org/wiki/File:John_Baskerville_by_James_Millar.jpg) | James Millar (floruit 1763-1805) | Public domain | 2400x2903 | 0.58 | dark coat: soft side fade |
| `pliego-mergenthaler.png` | Ottmar Mergenthaler | [Ottmar_Mergenthaler.jpg...](https://commons.wikimedia.org/wiki/File:Ottmar_Mergenthaler.jpg) | The original uploader was L-Cain at German Wikipedia. | Public domain | 3200x3062 | 0.32 | halftone source descreened (2.5 px blur); head only, bust is just the collar; head ~0.6 of tile |

## Hito

| Candidate | Figure | Source (Commons) | Author | Licence | Source px | Scale | Note |
|---|---|---|---|---|---|---|---|
| `hito-taylor.png` | Frederick Winslow Taylor | [Frederick_Taylor_1911.jpg...](https://commons.wikimedia.org/wiki/File:Frederick_Taylor_1911.jpg) | Rumely Company | Public domain | 1359x1887 | 0.91 | 1911 photo from a book scan (1359x1887); clean but a little grainy |
| `hito-gilbreth.png` | Lillian Moller Gilbreth | [Lillian_Moller_Gilbreth.jpg...](https://commons.wikimedia.org/wiki/File:Lillian_Moller_Gilbreth.jpg) | Smithsonian Institution from United States | No restrictions | 3221x3941 | 0.47 | head ~0.45 of tile |

## Dropped

- Forja - Nasmyth (forja-nasmyth.tif): the raised sleeve and the book come out as a large black mass at the left; no clean bust.
- Forja - Monge: skipped as instructed (licence/size).
- Trama - Lovelace (Claudet a), Jacquard a: sources too small (740 and 991 px wide).
- Pliego - Bodoni: sources too small (300-350 px), skipped as instructed.
- Hito - Gantt (hito-gantt-a 550x688, hito-gantt-b 371x563): the largest images on Commons are too small; a head at half tile width would need ~2-2.5x upscaling, so no candidate made.
- Hito - Taylor seated (hito-taylor.jpg 2408x3635): coarse halftone dither; coat edge and fade band show pixelation. Rendered as hito-taylor-seated but dropped.
- Hito - Taylor with signature plate (hito-taylor-b.jpg): clean cut-out but the source is soft (book scan); dropped for sharpness.
- Hito - Gilbreth b (833x1156): too small.
