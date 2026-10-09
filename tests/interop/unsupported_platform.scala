// On JavaScript the std's operations with no platform under them (reflection, the file system)
// throw UnsupportedOperationException, which a program catches like any other.
@main def main(): Unit =
  try println(classOf[String].getMethods.length)
  catch case e: UnsupportedOperationException => println("reflection: " + e.getMessage)
  try println(java.nio.file.Files.exists(java.nio.file.Paths.get("build.sbt")))
  catch case e: UnsupportedOperationException => println("files: " + e.getMessage)
