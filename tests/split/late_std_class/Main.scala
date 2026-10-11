package late

// A std class the typer completes after its final passes, the anonymous `InputStream` in the body
// of `GZIPInputStream`'s trailer reader, which the reach types: its `read()` overrides one
// alternative of the inherited overloaded `read`, which the std calls through `InputStream`.
@main def run(): Unit =
  println("label")
  val in: java.io.InputStream = new java.util.zip.GZIPInputStream(new java.io.ByteArrayInputStream(Array[Byte]()))
  println(in.read())
