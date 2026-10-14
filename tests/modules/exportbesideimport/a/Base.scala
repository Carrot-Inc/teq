package sharedscalajs

trait PreludeBase:
  export zio.{ZIO, Task}

object Prelude extends PreludeBase
