package jmb

// The inline call is compiled here, where its pickled body is read: its builder is the caller's.
def discarding(pb: ProcessBuilder): ProcessBuilder = jma.Sig.out(pb, ProcessBuilder.Redirect.DISCARD)

@main def run(): Unit =
  println(ProcessBuilder.Redirect.DISCARD.`type`())
