// The exceptions of the file system and of decoding (`java.nio.file`, `java.nio.charset`) and
// `UncheckedIOException`, under the JDK's names and hierarchy and with its messages, which the
// interpreter's natives throw (src/interp/files.rs); a file of their own, so that a program that
// reaches `IOException` enters these classes and not the file system's. On the JVM the classes
// are the JDK's.
package java.nio.file:

  // The message is the file, ` -> ` and the other file when there is one, `: ` and the reason
  // when there is one; the reason alone without a file.
  @jvmClass("java/nio/file/FileSystemException")
  class FileSystemException(file: String, other: String, reason: String) extends java.io.IOException(reason):
    def this(file: String) = this(file, null, null)
    def getFile: String = file
    def getOtherFile: String = other
    def getReason: String = reason
    override def getMessage: String =
      if file == null && other == null then reason
      else
        val sb = new java.lang.StringBuilder
        if file != null then sb.append(file)
        if other != null then sb.append(" -> ").append(other)
        if reason != null then sb.append(": ").append(reason)
        sb.toString

  @jvmClass("java/nio/file/NoSuchFileException")
  class NoSuchFileException(file: String, other: String, reason: String) extends FileSystemException(file, other, reason):
    def this(file: String) = this(file, null, null)

  @jvmClass("java/nio/file/FileAlreadyExistsException")
  class FileAlreadyExistsException(file: String, other: String, reason: String) extends FileSystemException(file, other, reason):
    def this(file: String) = this(file, null, null)

  @jvmClass("java/nio/file/AccessDeniedException")
  class AccessDeniedException(file: String, other: String, reason: String) extends FileSystemException(file, other, reason):
    def this(file: String) = this(file, null, null)

  @jvmClass("java/nio/file/NotDirectoryException")
  class NotDirectoryException(file: String) extends FileSystemException(file, null, null)

  @jvmClass("java/nio/file/DirectoryNotEmptyException")
  class DirectoryNotEmptyException(dir: String) extends FileSystemException(dir, null, null)

package java.nio.charset:

  @jvmClass("java/nio/charset/CharacterCodingException")
  class CharacterCodingException extends java.io.IOException(null)

  @jvmClass("java/nio/charset/MalformedInputException")
  class MalformedInputException(inputLength: Int) extends CharacterCodingException:
    def getInputLength: Int = inputLength
    override def getMessage: String = "Input length = " + inputLength

  @jvmClass("java/nio/charset/UnmappableCharacterException")
  class UnmappableCharacterException(inputLength: Int) extends CharacterCodingException:
    def getInputLength: Int = inputLength
    override def getMessage: String = "Input length = " + inputLength

package java.io:

  // Its cause is required, a null one a `NullPointerException`.
  @jvmClass("java/io/UncheckedIOException")
  class UncheckedIOException(message: String, cause: IOException) extends RuntimeException(message, cause):
    if cause == null then throw new NullPointerException()
    def this(cause: IOException) = this(if cause == null then null else cause.toString, cause)
    override def getCause: IOException = cause
