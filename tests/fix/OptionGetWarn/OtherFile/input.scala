/*
rule = OptionGetWarn
*/
package fix

class B extends ABase:
  val nonParamAlias: OI = None
  val opq: Opq = Opq()
  export A.otherFile as exported
  val local: Option[Int] = None

  def uses(): Unit =
    // A member of this file, then of A.scala: a val, a def, an inherited val.
    local.get // assert: OptionGetWarn
    A.otherFile.get // assert: OptionGetWarn
    A.otherFileDef.get // assert: OptionGetWarn
    otherFileInherited.get // assert: OptionGetWarn
    // A.scala's alias without parameters, its extension, an export's forwarder of its val, a local inferred from it.
    nonParamAlias.get // assert: OptionGetWarn
    opq.under.get // assert: OptionGetWarn
    exported.get // assert: OptionGetWarn
    val fromOther = A.otherFile
    fromOther.get // assert: OptionGetWarn
    ()
