package util

object Prelude:
  export lib.Impl.{*, given}
  export lib.Tags.{main => mainTag, cls => _, *}
  export react.{event, ReactEvent}
