# Matriz de compatibilidad — revisión 5

Esta matriz distingue reconocimiento sintético, observación de registros reales y modificación. No es una certificación de compatibilidad. No se realizaron pruebas físicas de iPhone para esta revisión.

| Tipo | Detección implementada | Personalización en iPhone | Prueba pendiente |
|---|---|---|---|
| Tarjeta de pago con arte estático | Ruta concreta o selección observada; tipo sigue sin confirmar si falta evidencia | Bloqueada: recuperación no verificable | Identificación real, arte original, restauración verificable |
| Apple Card | Contexto explícito marcado incompatible; ID opaco no revela emisor | Bloqueada; representación dinámica | Ninguna promesa de soporte mediante este método |
| Genérico / fidelidad / tienda | Eventos estructurados y tipo explícito; desconocido si no hay evidencia | Bloqueada | Eventos disponibles y disposición real de imágenes |
| Transporte | Contexto explícito de transporte; no se deduce por NFC | Bloqueada | Distinguir pase, credencial y arte sin tocar autenticación |
| Acceso / gimnasio | Contexto explícito de acceso; ID oculto permanece oculto | Bloqueada | Disponibilidad de eventos y recuperación segura del arte |
| Wallet con datos privados ocultos | Contadores de evento/redacción, sin inventar ID | Bloqueada | Determinar si iOS emite un ID utilizable |

Los pases de gimnasio y tienda no tienen una implementación certificada por emisor. No hay una regla que transforme automáticamente un pase de acceso o fidelidad en tarjeta bancaria.

| iOS / entorno | Evidencia disponible | Estado |
|---|---|---|
| Versiones anteriores a 27 | Formatos de syslog/unified y fixtures sintéticos | Sin prueba física de esta revisión |
| iOS 27 | Variantes Dashboard y protocolo de logs tomados de fuentes fijadas | Parsing probado con fixtures; sin certificación física |
| iOS 27.2 beta 3 o posterior | El README original advierte incompatibilidad del método de escritura | Sin soporte de escritura; no extrapolar detección |
| Versiones futuras | Ninguna garantía de estabilidad de servicios privados | Requieren investigación y fixtures nuevos |
| Windows runner de Actions | Test, build, PE, self-check y arranque automatizados cuando CI termina correctamente | Es un ensayo de PC, no del teléfono |
| Windows 10 físico | No ensayado para esta revisión | Pendiente |
| Windows 11 físico + iPhone | No ensayado para esta revisión | Pendiente |

## Vía necesaria para habilitar modificaciones

Antes de habilitar cualquier escritor se necesita demostrar la adquisición del arte original, una copia completa vinculada al dispositivo/pase correcto, validación de archivos y recuperación verificable del destino (incluidos los estados auxiliares que se modificarían). Se prefiere una lectura no destructiva. Un candidato que traslade originales necesita guardar una intención duradera de recuperación antes del primer traslado y demostrar recuperación tras interrupción y reconexión en un dispositivo físico. Una copia local con hash, un traslado temporal o una confirmación del servicio por sí solos no demuestran esa recuperación. Si la imagen pertenece a un paquete firmado, también debe respetarse la firma del emisor; no se extraen claves ni credenciales para resolverlo.


## Editor local de la revisión 5

Los formatos clásicos generic, storeCard, coupon, boardingPass y eventTicket tienen selección de imágenes y exportación PNG probadas con paquetes sintéticos. NFC declarado se informa sin interpretar sus credenciales. Esto es compatibilidad de archivo local, no detección USB ni modificación del pase instalado. Poster, estilos modernos y recursos localizados se rechazan explícitamente. Ver [LOCAL-PASS-EDITOR.md](LOCAL-PASS-EDITOR.md).
