// The exceptions of streams, sockets, archives, digests and processes (`java.io`, `java.net`,
// `java.util.zip`, `java.security`, `java.lang`), under the JDK's names and hierarchy, which the
// interpreter's natives throw (src/interp/process.rs, net.rs, archive.rs); a file of their own, as
// nio_exceptions.scala is, so that a program that reaches `Exception` enters these classes and not
// io.scala's, socket.scala's, zip.scala's or process.scala's. On the JVM the classes are the JDK's.
package java.io:

  @jvmClass("java/io/FileNotFoundException")
  class FileNotFoundException(message: String = null) extends IOException(message)

  @jvmClass("java/io/InterruptedIOException")
  class InterruptedIOException(message: String = null) extends IOException(message)

package java.net:

  @jvmClass("java/net/SocketException")
  class SocketException(message: String = null) extends java.io.IOException(message)

  @jvmClass("java/net/ConnectException")
  class ConnectException(message: String = null) extends SocketException(message)

  @jvmClass("java/net/BindException")
  class BindException(message: String = null) extends SocketException(message)

  @jvmClass("java/net/SocketTimeoutException")
  class SocketTimeoutException(message: String = null) extends java.io.InterruptedIOException(message)

  @jvmClass("java/net/UnknownHostException")
  class UnknownHostException(message: String = null) extends java.io.IOException(message)

package java.util.zip:

  @jvmClass("java/util/zip/ZipException")
  class ZipException(message: String = null) extends java.io.IOException(message)

  @jvmClass("java/util/zip/DataFormatException")
  class DataFormatException(message: String = null) extends Exception(message)

package java.security:

  @jvmClass("java/security/GeneralSecurityException")
  class GeneralSecurityException(message: String = null) extends Exception(message)

  @jvmClass("java/security/NoSuchAlgorithmException")
  class NoSuchAlgorithmException(message: String = null) extends GeneralSecurityException(message)

package java.lang:

  @jvmClass("java/lang/IllegalThreadStateException")
  class IllegalThreadStateException(message: String = null) extends IllegalArgumentException(message)
