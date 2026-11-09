# Resultados de la revisión 5 — editor local de pases

Fecha: 2026-11-09. Código compilado: `25b147124971d23950d2427e57354f4ad7701e22`. Rama: `research/pass-artwork`.

**Entregable parcial respecto al objetivo NFC:** nuevo ejecutable con importación local de imágenes de pases clásicos y exportación PNG. No habilita la modificación de pases instalados en el iPhone.

## Descarga y comprobación

[Descargar AirCard-Safe-Windows-x64](https://github.com/Ronald234/AirCard-Safe/actions/runs/42556330570/artifacts/13647991967). Extraer en una carpeta nueva y abrir `AirCard-Safe.exe`. No requiere herramientas de desarrollo. GitHub puede pedir iniciar sesión; el artifact vence el 2026-12-09.

SHA-256 del **EXE**, verificado en el runner tras compilar:

```text
a39b4a1a90dfb243bd019c43a3a3b4b8f9f1ef2c033f2fe1451e6fd384d804df
```

SHA-256 del archivo del artifact ZIP, distinto del EXE:
`882dcb8bca40ba3c178d8a270eb43c5a4522d31cb9ad0d938be32e448fd79677`.

`Get-FileHash .\AirCard-Safe.exe -Algorithm SHA256` debe coincidir con el hash del EXE y con `SHA256SUMS.txt`. Ver instrucciones en [LOCAL-PASS-EDITOR.md](LOCAL-PASS-EDITOR.md).

## Evidencia del ejecutable entregado

[Windows Actions run 42556330570](https://github.com/Ronald234/AirCard-Safe/actions/runs/42556330570), job 128962751740, **success**:

- `cargo fmt --all -- --check`: aprobado.
- `cargo test --locked`: 138 ejecuciones aprobadas, 0 fallos, 2 omitidas. Las dos omitidas son la enumeración USB física incluida en dos targets; requieren iPhone y controladores.
- `cargo clippy --locked --all-targets -- -D clippy::correctness`: aprobado; existen advertencias no fatales, no se afirma ausencia de advertencias.
- `cargo build --locked --release`: aprobado con CRT estático.
- 8 pruebas del verificador PE: aprobadas.
- Binario real: cabeceras MZ/PE, PE32+, AMD64, aplicación Windows GUI verificadas.
- `--self-check` y arranques breves de GUI debug/release: aprobados.
- Artifact publicado: 13647991967, 5,927,278 bytes de ZIP; contiene EXE, hashes, build-info, licencia, documentación y logs.

[Research run 42556330648](https://github.com/Ronald234/AirCard-Safe/actions/runs/42556330648): pruebas completas de la biblioteca experimental, formato, Clippy y build release aprobados en Windows (23 ejecuciones) y Linux (21). Estas cifras son ejecuciones por target y no casos únicos.

## Cambios comprobados

- Inventario gráfico clásico compartido entre biblioteca experimental y aplicación.
- Importación acotada de paquetes sin extracción al disco ni modificación del original.
- Rechazo de rutas ambiguas, recursos no admitidos, imágenes inválidas y límites excedidos.
- Solo imágenes recodificadas y metadatos mínimos sobreviven a la importación; no se exportan campos NFC, firma ni autenticación.
- Reemplazo PNG con dimensiones originales, recorte centrado y comprobación de píxeles.
- Prueba sobre archivo temporal confirma que abrir y preparar una imagen deja idéntico el paquete original.
- Una revisión independiente detectó una asignación intermedia potencialmente enorme al redimensionar: corregida recortando antes de redimensionar. Regresión de proporciones extremas observada fallar y después pasar.
- Vista previa anterior eliminada al preparar otra imagen; regresión observada fallar y después pasar.

## Pruebas pendientes y bloqueos

No se realizaron pruebas físicas de iPhone, acceso NFC, pago, restauración ni Windows 10 físico. El arranque automatizado no es una prueba completa de interacción con todos los controles.

No se ha demostrado adquisición y recuperación segura de las imágenes de un pase instalado por USB en Windows. El editor requiere que el usuario ya disponga del archivo local. No extrae el pase desde Wallet, no reconstruye un paquete firmado y no modifica credenciales. Aplicar/restaurar al teléfono siguen bloqueados en interfaz y backend. Por tanto, la meta de editar imágenes de pases NFC instalados continúa **sin resolver**.
