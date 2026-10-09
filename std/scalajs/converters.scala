// `import JSConverters.*` brings the extensions; `import JSConverters.JSRichOption` (the names of the original's
// implicit classes) brings a given whose object holds the same extension, which the compiler searches as well.
package scala.scalajs.js

object JSConverters:
  extension [A](self: Option[A])
    def orUndefined: UndefOr[A] = self match
      case Some(a) => a
      case None => undefined

  extension [A](self: IterableOnce[A])
    @js("Array.from($0)")
    def toJSArray: Array[A]
    @js("Array.from($0)")
    def toJSIterable: Iterable[A]

  extension [A](self: Map[String, A])
    def toJSDictionary: Dictionary[A] = Dictionary(self.toList*)

  object JSRichOptionOps:
    extension [A](self: Option[A])
      def orUndefined: UndefOr[A] = self match
        case Some(a) => a
        case None => undefined

  object JSRichIterableOnceOps:
    extension [A](self: IterableOnce[A])
      @js("Array.from($0)")
      def toJSArray: Array[A]
      @js("Array.from($0)")
      def toJSIterable: Iterable[A]

  object JSRichMapOps:
    extension [A](self: Map[String, A])
      def toJSDictionary: Dictionary[A] = Dictionary(self.toList*)

  given JSRichOption: JSRichOptionOps.type = JSRichOptionOps
  given JSRichIterableOnce: JSRichIterableOnceOps.type = JSRichIterableOnceOps
  given JSRichIterable: JSRichIterableOnceOps.type = JSRichIterableOnceOps
  given JSRichMap: JSRichMapOps.type = JSRichMapOps
