# AirCard Safe — revisión 4

Aplicación nativa de Windows para diagnosticar la detección de Apple Wallet por **USB** y preparar imágenes localmente. Código abierto MIT, derivado de [Lumid-Off/AirCard-Windows](https://github.com/Lumid-Off/AirCard-Windows), con comparación técnica del [fork de dhava-gautama](https://github.com/dhava-gautama/AirCard-Windows).

**Esta revisión bloquea aplicar y restaurar imágenes en el iPhone.** No existe un método verificado de respaldar el arte protegido y comprobar su recuperación usando el AFC normal. Ni aceptar un aviso ni aportar archivos con SHA-256 elimina esa limitación. El método anterior de escritura sin respaldo deja de estar disponible. No se alteran cachés, Books, pases, credenciales NFC ni mecanismos de autenticación.

## Descargar el ejecutable

**[Descargar revisión 4 verificada](https://github.com/Ronald234/AirCard-Safe/actions/runs/41754963135/artifacts/13288007407)**. La ejecución de Windows terminó correctamente; el SHA-256 y los resultados están en [STATUS.md](docs/STATUS.md).

En [Actions: AirCard Safe Windows](https://github.com/Ronald234/AirCard-Safe/actions/workflows/build.yml), abre la ejecución correcta de esta revisión que termine en **success** y descarga **AirCard-Safe-Windows-x64**. Extrae el ZIP y abre **AirCard-Safe.exe**. El paquete incluye `SHA256SUMS.txt`, `build-info.json`, los resultados de pruebas y documentación. Los artifacts caducan a los 30 días; GitHub puede exigir iniciar sesión.

Verifica en PowerShell: `Get-FileHash .\AirCard-Safe.exe -Algorithm SHA256`. Debe coincidir con `SHA256SUMS.txt`. El SHA-256 comprueba integridad; el programa no tiene firma de código.

El usuario no necesita Python, Git, Rust, Cargo ni Visual Studio. Para conectarse al iPhone necesita los controladores y bibliotecas oficiales de Apple (Apple Devices/iTunes según la instalación). No se redistribuyen DLL de Apple ni se descargan dependencias de desarrollo. Windows 10/11 x64 es el objetivo; una compilación en CI no certifica todas esas instalaciones.

## Uso

1. Conecta el iPhone desbloqueado por USB y acepta Confiar. Safe rechaza rutas Wi-Fi o ambiguas; puede ser necesario desactivar sincronización por Wi-Fi.
2. Pulsa **Comprobar soporte Apple** y **Buscar iPhone por USB**.
3. Inicia **Diagnosticar tarjetas y pases**. Abre Wallet y selecciona el pase. La captura se limita a unos dos minutos, con cancelación; una llamada privada de Apple que se bloquee puede exceder el plazo.
4. Revisa las observaciones y el resumen. Solo se clasifica un pase cuando existe evidencia explícita en los eventos. Una ruta o un identificador no demuestra compatibilidad de imagen ni NFC.
5. Si necesitas compartir el diagnóstico, guárdalo como JSON. Contiene solo contadores y estados fijos: no contiene registros originales, identificadores, nombres, rutas, códigos ni datos del dispositivo. No se sube automáticamente.
6. Elige una imagen y ajusta el recorte. Puedes exportar un ZIP local con PNG @2x/@3x, PDF y SHA-256. **Ese ZIP no es un pase firmado ni un respaldo y no se instala en Wallet.** No se sobrescriben archivos existentes.

Aplicar y restaurar permanecen deshabilitados tanto en la interfaz como en el backend. No hay una opción para ignorar la protección. Las funciones de modificación del teclado de contraseña no están incluidas.

## Detección y límites

Se añaden los eventos Dashboard encontrados en el fork y clasificación explícita genérica, fidelidad, transporte y acceso, además de candidatos desconocidos. Los datos privados ocultos por iOS no se recuperan ni se inventan. Los eventos ausentes no prueban que el pase no exista.

No se declara funcionamiento físico de esta revisión con tarjetas bancarias, pases de gimnasio, tiendas ni NFC. La compatibilidad por tipo/iOS y las pruebas pendientes están en [COMPATIBILITY.md](docs/COMPATIBILITY.md). Apple Card usa una representación dinámica que el método de imágenes estáticas no soporta.

El respaldo del fork mueve originales, acepta capturas parciales y puede borrar una copia antes de recuperarla. **No se integró ese respaldo.** La negociación AirTraffic/Grappa se comparó mediante revisión de código. El formato de archivo agrupado conserva pruebas offline, pero el escritor queda fuera del ejecutable: una sincronización exitosa no prueba recuperación. Consulta [RECOVERY-AUDIT.md](docs/RECOVERY-AUDIT.md) y [TRANSPORT-AUDIT.md](docs/TRANSPORT-AUDIT.md).

## Pruebas y compilación

GitHub Actions usa `windows-latest`, `cargo test --locked`, análisis Clippy y `cargo build --locked --release`. Antes de publicar verifica PE32+ AMD64 Windows GUI, `--self-check` y arranque breve sin iPhone. El artifact contiene los logs y hash del binario realmente compilado. No confundir estas comprobaciones con pruebas físicas de restauración, pago o acceso al gimnasio.

Ver [TESTING.md](docs/TESTING.md), [SECURITY-REVIEW.md](docs/SECURITY-REVIEW.md), [SCANNER-DIAGNOSTICS.md](docs/SCANNER-DIAGNOSTICS.md) y [STATUS.md](docs/STATUS.md). Los documentos del método anterior se conservan como historial y **no son instrucciones para esta revisión**.

## Referencias y licencia

- Original Windows: `88131e301c88fd9238cbeff9d091967c3ddbf10f`.
- Fork comparado: `8df5b1664aff7e817b90ad11a15eb8a6ebbef0e4`.
- Base Safe anterior: `f966a21cb6f8f0c45ede5f7ae71c9ce931d08f76` (historia remota).
- MIT original conservada en LICENSE; atribución adicional del decodificador nativo en [UNIFIED_SCANNER.md](docs/UNIFIED_SCANNER.md), incluida en el artifact.

La identidad Cargo permanece `aircard 1.2.4` por continuidad con la base; el binario se llama `AirCard-Safe.exe`. Cargo.lock se actualiza para ZIP 9.3.0 tras revisar sus avisos de seguridad. Los números de revisión describen los cambios de Safe.
