// The slice of org.scalajs.dom that the facade files name.
package org.scalajs.dom

import scala.scalajs.js

@js.native
@JSGlobal("EventTarget")
class EventTarget extends js.Object:
  def addEventListener[T <: Event](tpe: String, listener: js.Function1[T, ?], useCapture: Boolean = js.native): Unit =
    js.native

@js.native
@JSGlobal("Node")
class Node extends EventTarget:
  def textContent: String = js.native
  def appendChild(child: Node): Node = js.native

@js.native
@JSGlobal("Element")
class Element extends Node:
  def tagName: String = js.native
  def setAttribute(name: String, value: String): Unit = js.native

@js.native
@JSGlobal("HTMLElement")
class HTMLElement extends Element:
  var title: String = js.native
  def focus(): Unit = js.native

@js.native
@JSGlobal("HTMLInputElement")
class HTMLInputElement extends HTMLElement:
  var value: String = js.native

@js.native
@JSGlobal("HTMLDivElement")
class HTMLDivElement extends HTMLElement

@js.native
@JSGlobal("Event")
class Event extends js.Object:
  def `type`: String = js.native
  def preventDefault(): Unit = js.native

@js.native
@JSGlobal("MouseEvent")
class MouseEvent extends Event:
  def clientX: Double = js.native
  def clientY: Double = js.native

@js.native
@JSGlobal("Blob")
class Blob extends js.Object:
  def size: Double = js.native
  def `type`: String = js.native

@js.native
@JSGlobal("File")
class File extends Blob:
  def name: String = js.native

object html:
  type Div = HTMLDivElement
  type Input = HTMLInputElement
