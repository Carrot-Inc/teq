package rounds

import inlinebodies.{Bodies, Tag}

@main def run(): Unit =
  println(s"${Reads.made} ${Reads.named} ${Reads.built} ${Reads.fixed} ${Reads.after}")
  println(s"${Bodies.made(8)} ${Tag.derived(6)}")
