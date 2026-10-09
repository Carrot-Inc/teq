package mnb

import mna.Cls
import mna.Cls.tw

def labeled(label: String = "", className: String = ""): String = label + "|" + className

@main def run(): Unit =
  println(labeled(className = Cls.classes("p-4", "flex")))
  println(labeled(className = tw"max-w-xs mx-auto"))
