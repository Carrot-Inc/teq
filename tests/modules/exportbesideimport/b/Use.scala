package apiclient

import util.Prelude.{*, given}
import zio.*

object Use:
  def main(args: Array[String]): Unit =
    println(ZIO.succeed(extra + 41))
