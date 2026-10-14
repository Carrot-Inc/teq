package scala.util

/** scala-library's: the command line parsing a typed `@main` method's proxy calls (dotty's `MainProxies`). */
object CommandLineParser:

  /** An exception raised for an illegal command line: the faulty argument's index, from 0, and the message. */
  class ParseError(val idx: Int, val msg: String) extends Exception

  /** The command line argument `str`, of index `n`, as a `T`: `ParseError` where it is none. */
  def parseString[T](str: String, n: Int)(using fs: FromString[T]): T =
    try fs.fromString(str)
    catch case ex: IllegalArgumentException => throw ParseError(n, ex.toString)

  /** The `n`th argument of `args`, from 0, as a `T`: `ParseError` where there is none or it is no `T`. */
  def parseArgument[T](args: Array[String], n: Int)(using fs: FromString[T]): T =
    if n < args.length then parseString(args(n), n)
    else throw ParseError(n, "more arguments expected")

  /** The arguments from the `n`th on, each a `T`. */
  def parseRemainingArguments[T](args: Array[String], n: Int)(using fs: FromString[T]): List[T] =
    if n < args.length then parseString(args(n), n) :: parseRemainingArguments(args, n + 1)
    else Nil

  /** Prints the message of a `ParseError`. */
  def showError(err: ParseError): Unit =
    val where =
      if err.idx == 0 then ""
      else if err.idx == 1 then " after first argument"
      else s" after ${err.idx} arguments"
    println(s"Illegal command line$where: ${err.msg}")

  trait FromString[T]:
    /** May throw `IllegalArgumentException`. */
    def fromString(s: String): T

    def fromStringOption(s: String): Option[T] =
      try Some(fromString(s))
      catch case ex: IllegalArgumentException => None

  object FromString:
    given FromString[String] with
      def fromString(s: String) = s

    given FromString[Boolean] with
      def fromString(s: String) = s.toBoolean

    given FromString[Byte] with
      def fromString(s: String) = s.toByte

    given FromString[Short] with
      def fromString(s: String) = s.toShort

    given FromString[Int] with
      def fromString(s: String) = s.toInt

    given FromString[Long] with
      def fromString(s: String) = s.toLong

    given FromString[Float] with
      def fromString(s: String) = s.toFloat

    given FromString[Double] with
      def fromString(s: String) = s.toDouble
