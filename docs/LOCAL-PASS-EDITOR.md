# Editor local de imágenes de pases — revisión 5

Esta revisión añade una función de escritorio utilizable: abrir un archivo `.pkpass` que ya tengas, ver sus recursos gráficos y preparar un PNG de reemplazo con las dimensiones originales. **No cumple todavía el objetivo de modificar la imagen de un pase NFC instalado en el iPhone.** Las escrituras y restauraciones por USB siguen bloqueadas.

## Uso

1. Abre `AirCard-Safe.exe`; no requiere Python, Rust ni Git.
2. Elige la imagen PNG/JPG/WebP que quieres utilizar en la sección de imágenes.
3. En **Imágenes de un pase local**, abre un `.pkpass` y selecciona un recurso.
4. Pulsa **Preparar reemplazo**. La vista previa usa un recorte centrado con las dimensiones del recurso; los controles de enfoque del diseño bancario de arriba no se aplican aquí.
5. Guarda el reemplazo como un PNG nuevo. El nombre de destino debe ser nuevo: no se sobrescriben archivos.

No necesitas conectar un iPhone para esta función. Si solo tienes el pase instalado en Wallet y no tienes su archivo, este editor no puede obtenerlo del teléfono. No solicita que subas pases ni registros.

## Alcance

| Entrada | Recursos locales admitidos |
| --- | --- |
| Generic clásico | logo, thumbnail |
| StoreCard / Coupon clásicos | logo, strip |
| BoardingPass clásico | logo, footer |
| EventTicket clásico | logo, strip o background/thumbnail |
| Variantes @2x y @3x | Solo si existen en el paquete |
| Poster / estilos modernos / recursos localizados | No admitidos |
| Credenciales NFC y autenticación | No se muestran, exportan ni modifican |
| Pase instalado en iPhone / Apple Pay | Sin edición directa |

La presencia del diccionario NFC es solo una declaración del paquete, no una prueba de validez, de tecnología de acceso ni de compatibilidad con un lector. No se confirma compatibilidad con ningún gimnasio, tienda o versión de iOS mediante estas pruebas sintéticas.

## Seguridad y límites

- Entrada de solo lectura, sin extracción del ZIP al sistema de archivos.
- Hasta 8 MiB comprimidos, 512 entradas, 4 MiB por entrada y 32 MiB declarados en total.
- Metadatos hasta 256 KiB; imágenes hasta 4096 píxeles por lado y 16 millones de píxeles en conjunto.
- Rechaza enlaces, cifrado, directorios, rutas ambiguas y variantes de nombres que colisionan.
- Analiza metadatos temporalmente; conserva únicamente el tipo de diseño, la presencia de NFC y las imágenes recodificadas. No se registra ni exporta el contenido NFC, números de serie, autenticación, firma ni JSON del pase.
- Las imágenes pueden contener información personal visible. El usuario decide qué PNG guardar. No existe garantía de borrado seguro de la memoria.
- No verifica la firma ni la procedencia del pase. No reconstruye ni firma un `.pkpass`: un PNG exportado no es un pase instalable, un respaldo ni autorización de escritura al teléfono.
- Un fallo de guardado puede dejar un PNG parcial; se informa y no se borra automáticamente.

## Bloqueo pendiente del objetivo principal

Falta implementar y verificar en Windows la obtención de las imágenes originales del pase instalado, la recuperación persistente antes de cada cambio, la lectura posterior del destino y la restauración tras interrupciones. El inventario local no demuestra que dichas imágenes sean accesibles por USB. No se desactiva esa protección para aparentar edición NFC.

## Verificación

La revisión independiente encontró y corrigió un riesgo de asignación excesiva de memoria: ahora se recorta antes de redimensionar. También se elimina la textura anterior al preparar otro reemplazo para que no aparezca etiquetada como original.

Las pruebas usan paquetes sintéticos, incluyendo marcadores de credenciales ficticias. Comprueban selección por diseño, rechazo de archivos inválidos, límites, conservación del archivo original y exportación de píxeles con dimensiones originales. GitHub Actions ejecuta `cargo test --locked`, `cargo build --locked --release`, análisis estático, verificación PE y arranque de la GUI. Consultar el resultado del run correspondiente; un arranque en el runner no equivale a prueba física de iPhone ni a una prueba completa de interfaz.
