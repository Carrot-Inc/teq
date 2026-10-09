import java.nio.file.{Files, Paths}
import scala.quoted.*

// A macro reads files at compile time as under scalac and writes none: its expansions run on the
// parallel typer's workers and may be run again, so that its effects on disk would have no order. The
// write is refused, naming why, and nothing is written (target/ of the working directory keeps no
// macro_files_write).
object Macros:
  inline def written: String = ${ writtenImpl }

  def writtenImpl(using Quotes): Expr[String] =
    Files.writeString(Paths.get("target/macro_files_write.txt"), "written")
    Expr(Files.readString(Paths.get("target/macro_files_write.txt")))
