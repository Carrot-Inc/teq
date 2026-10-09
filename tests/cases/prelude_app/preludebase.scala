package sharedlib

trait PreludeCore:
  export sharedlib.tailwind.Tw
  export sharedlib.tailwind.TailwindSyntax.*
  export sharedlib.vdom.{TagMod, Element, Text, Key, ClassArg, Css}
  export sharedlib.vdom.HtmlTags.*
  export sharedlib.vdom.HtmlAttrs.*
  export sharedlib.vdom.VdomSyntax.*
  export sharedlib.react.Component
  export sharedlib.react.Hooks.*

  // Defined in the trait itself, so it is reached through the object that mixes the trait in.
  extension (sc: StringContext)
    def css(args: Any*): Css = Css(sc.s(args*))
