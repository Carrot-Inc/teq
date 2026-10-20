// lila: team/Team.scala:117/118/120 (extension (idsStr: IdsStr) { def toArray: Array[TeamId] = ...;
// def toList = idsStr.nonEmpty.so(toArray.toList); ... def nonEmpty = idsStr.nonEmpty }) -- related;
// on lila teq reports "recursive use of toList/toSet/nonEmpty", here the same lookup shows as a mismatch.
// Inside the extension, `toArray` means `idsStr.toArray`; String has no member toArray, so the
// extension method in scope is tried before any implicit conversion (Typer.tryExtensionOrConversion):
// toArray is the Array[Int] one, not StringOps.toArray, and toList is List[Int].
object Team:
  opaque type IdsStr = String
  object IdsStr:
    def apply(s: String): IdsStr = s
    extension (idsStr: IdsStr)
      def toArray: Array[Int] = idsStr.split(' ').map(_.length)
      def toList = toArray.toList

object Use:
  def lengths(idsStr: Team.IdsStr): List[Int] = idsStr.toList

@main def run() = println(Use.lengths(Team.IdsStr("ab c")))
