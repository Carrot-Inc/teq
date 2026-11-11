//> using scala 3.8.4
//> using dep org.scala-lang::scala3-compiler:3.8.4
//> using dep org.scala-js::scalajs-ir:1.22.0

// Canon <file>...: what a .tasty or a .sjsir file holds, apart from how it is encoded, for tests/modules-scalac-split.sh
// to tell scalac's two builds apart by: a pickle's trees with each shared reference written out where it stands,
// names as their text and constants as their values, no address, length or position (TastyFormat's categories of
// tags, as dotty's TastyPrinter reads them): a reference by address is to what stands there, a definition by its tag
// and name, a lambda type's parameter by its binder's tag and its name; a Scala.js IR class as scalajs-ir prints it,
// no position.
// Each file's text follows a line `=== <file>`.
import dotty.tools.tasty.{TastyReader, TastyBuffer, TastyFormat}
import dotty.tools.tasty.TastyFormat.*
import dotty.tools.dotc.core.tasty.TastyUnpickler
import dotty.tools.dotc.core.tasty.TastyUnpickler.NameTable

object Canon:
  def main(args: Array[String]): Unit =
    for f <- args do
      println(s"=== $f")
      val bytes = java.nio.file.Files.readAllBytes(java.nio.file.Paths.get(f))
      if f.endsWith(".tasty") then println(tasty(bytes))
      else if f.endsWith(".sjsir") then println(sjsir(bytes))

  private def sjsir(bytes: Array[Byte]): String =
    org.scalajs.ir.Serializers.deserialize(java.nio.ByteBuffer.wrap(bytes)).show

  private def tasty(bytes: Array[Byte]): String =
    val unpickler = new TastyUnpickler(bytes)
    val out = new StringBuilder
    unpickler.unpickle(new TastyUnpickler.SectionUnpickler[Unit](ASTsSection) {
      def unpickle(reader: TastyReader, names: NameTable): Unit = trees(reader, names, out)
    })
    out.toString

  private def trees(reader: TastyReader, names: NameTable, out: StringBuilder): Unit =
    import reader.*
    var indent = 0
    def line(s: String): Unit = out.append("  " * indent).append(s).append('\n')
    def name(): String = names(TastyBuffer.NameRef(readNat())).toString
    def at[T](addr: TastyBuffer.Addr)(body: => T): T =
      val back = currentAddr
      goto(addr)
      try body finally goto(back)
    def skip(): Unit =
      val tag = readByte()
      if tag >= firstLengthTreeTag then goto(readEnd())
      else if tag >= firstNatASTTreeTag then { readNat(); skip() }
      else if tag >= firstASTTreeTag then skip()
      else if tag >= firstNatTreeTag then readNat()
    // What the address of a definition (TERMREFdirect, TERMREFsymbol, TYPEREFdirect, TYPEREFsymbol, RETURN's method)
    // or of a recursive type (RECthis) names.
    def target(): String = at(TastyBuffer.Addr(readNat())) {
      val tag = readByte()
      if tag >= firstLengthTreeTag then { readEnd(); s"${astTagToString(tag)} ${name()}" } else astTagToString(tag)
    }
    // PARAMtype's parameter `num` of the lambda type at its binder's address (`result_Type TypesNames`).
    def parameter(binder: TastyBuffer.Addr, num: Int): String = at(binder) {
      val tag = readByte()
      val end = readEnd()
      skip()
      var i = 0
      var found = ""
      while currentAddr.index < end.index && !isModifierTag(nextByte) && i <= num do
        skip()
        val n = name()
        if i == num then found = n
        i += 1
      s"${astTagToString(tag)} $found"
    }
    def tree(): Unit =
      val tag = readByte()
      val tagName = astTagToString(tag)
      if tag >= firstLengthTreeTag then
        val end = readEnd()
        def rest(): Unit = while currentAddr.index < end.index do tree()
        tag match
          case RENAMED => line(s"$tagName ${name()} ${name()}")
          case VALDEF | DEFDEF | TYPEDEF | TYPEPARAM | PARAM | NAMEDARG | BIND =>
            line(s"$tagName ${name()}"); indent += 1; rest(); indent -= 1
          case REFINEDtype | TERMREFin | TYPEREFin | SELECTin =>
            line(s"$tagName ${name()}"); indent += 1; tree(); rest(); indent -= 1
          case RETURN => line(s"$tagName ${target()}"); indent += 1; rest(); indent -= 1
          case HOLE => line(s"$tagName ${readNat()}"); indent += 1; rest(); indent -= 1
          case METHODtype | POLYtype | TYPELAMBDAtype =>
            line(tagName); indent += 1; tree()
            while currentAddr.index < end.index && !isModifierTag(nextByte) do { tree(); line(name()) }
            rest(); indent -= 1
          case PARAMtype =>
            val binder = TastyBuffer.Addr(readNat())
            val num = readNat()
            line(s"$tagName ${parameter(binder, num)} $num")
          case _ => line(tagName); indent += 1; rest(); indent -= 1
        goto(end)
      else if tag >= firstNatASTTreeTag then
        tag match
          case IDENT | IDENTtpt | SELECT | SELECTtpt | TERMREF | TYPEREF | SELFDEF => line(s"$tagName ${name()}")
          case TERMREFsymbol | TYPEREFsymbol => line(s"$tagName ${target()}")
          case _ => line(s"$tagName ${readNat()}")
        indent += 1; tree(); indent -= 1
      else if tag >= firstASTTreeTag then
        line(tagName); indent += 1; tree(); indent -= 1
      else if tag >= firstNatTreeTag then
        tag match
          case SHAREDterm | SHAREDtype =>
            // The shared tree written out where it stands, as its reader reads it there (`forkAt`).
            val at = TastyBuffer.Addr(readNat())
            val back = currentAddr
            goto(at); tree(); goto(back)
          case TERMREFpkg | TYPEREFpkg | STRINGconst | IMPORTED => line(s"$tagName ${name()}")
          case TERMREFdirect | TYPEREFdirect | RECthis => line(s"$tagName ${target()}")
          case BYTEconst | SHORTconst | CHARconst | INTconst | LONGconst | FLOATconst | DOUBLEconst =>
            line(s"$tagName ${readLongInt()}")
          case _ => line(s"$tagName ${readNat()}")
      else line(tagName)
    while !isAtEnd do tree()
