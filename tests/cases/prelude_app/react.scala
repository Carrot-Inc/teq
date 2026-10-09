//> using dep com.lihaoyi::sourcecode:0.4.2
// jars: sourcecode
package sharedlib.react

import sharedlib.vdom.Element

final case class Component(displayName: String, line: Int, render: () => Element)

object Hooks:
  def fc(body: => Element)(using name: sourcecode.FullName, line: sourcecode.Line): Component =
    Component(name.value, line.value, () => body)

  def useDebugLabel(prefix: String)(using name: sourcecode.Name): String =
    prefix + ":" + name.value
