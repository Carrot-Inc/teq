package org.scalajs.dom

import scala.scalajs.js
import scala.scalajs.js.annotation.{JSBracketAccess, JSGlobal}

/** The array-like collections of the DOM; DOMListOps.scala gives them the Seq reading of the original's implicit conversion. */
@js.native
trait DOMList[+T] extends js.Object:
  def length: Int = js.native
  @JSBracketAccess
  def apply(index: Int): T = js.native

@js.native
@JSGlobal("NodeList")
class NodeList[+T <: Node] extends DOMList[T]:
  def item(index: Int): T = js.native

@js.native
@JSGlobal("HTMLCollection")
class HTMLCollection[+E <: Element] extends DOMList[E]:
  def item(index: Int): E = js.native
  def namedItem(name: String): E = js.native

@js.native
@JSGlobal("DOMTokenList")
class DOMTokenList extends DOMList[String]:
  def item(index: Int): String = js.native
  def contains(token: String): Boolean = js.native
  def add(tokens: String*): Unit = js.native
  def remove(tokens: String*): Unit = js.native
  def toggle(token: String, force: Boolean = js.native): Boolean = js.native
  def replace(oldToken: String, newToken: String): Boolean = js.native
  var value: String = js.native

@js.native
@JSGlobal("Node")
class Node extends EventTarget:
  def nodeType: Int = js.native
  def nodeName: String = js.native
  var nodeValue: String = js.native
  var textContent: String = js.native
  def baseURI: String = js.native
  def isConnected: Boolean = js.native
  def ownerDocument: Document = js.native
  def parentNode: Node = js.native
  def childNodes: NodeList[Node] = js.native
  def firstChild: Node = js.native
  def lastChild: Node = js.native
  def previousSibling: Node = js.native
  def nextSibling: Node = js.native
  def hasChildNodes(): Boolean = js.native
  def appendChild(newChild: Node): Node = js.native
  def removeChild(oldChild: Node): Node = js.native
  def insertBefore(newChild: Node, refChild: Node): Node = js.native
  def replaceChild(newChild: Node, oldChild: Node): Node = js.native
  def cloneNode(deep: Boolean = js.native): Node = js.native
  def contains(otherNode: Node): Boolean = js.native
  def compareDocumentPosition(other: Node): Int = js.native
  def isSameNode(other: Node): Boolean = js.native
  def isEqualNode(other: Node): Boolean = js.native
  def normalize(): Unit = js.native
  def getRootNode(): Node = js.native

@js.native
@JSGlobal("Node")
object Node extends js.Object:
  val ELEMENT_NODE: Int = js.native
  val ATTRIBUTE_NODE: Int = js.native
  val TEXT_NODE: Int = js.native
  val CDATA_SECTION_NODE: Int = js.native
  val PROCESSING_INSTRUCTION_NODE: Int = js.native
  val COMMENT_NODE: Int = js.native
  val DOCUMENT_NODE: Int = js.native
  val DOCUMENT_TYPE_NODE: Int = js.native
  val DOCUMENT_FRAGMENT_NODE: Int = js.native
  val DOCUMENT_POSITION_DISCONNECTED: Int = js.native
  val DOCUMENT_POSITION_PRECEDING: Int = js.native
  val DOCUMENT_POSITION_FOLLOWING: Int = js.native
  val DOCUMENT_POSITION_CONTAINS: Int = js.native
  val DOCUMENT_POSITION_CONTAINED_BY: Int = js.native
  val DOCUMENT_POSITION_IMPLEMENTATION_SPECIFIC: Int = js.native

@js.native
@JSGlobal("Element")
class Element extends Node:
  def tagName: String = js.native
  def localName: String = js.native
  def namespaceURI: String = js.native
  def prefix: String = js.native
  var id: String = js.native
  var className: String = js.native
  def classList: DOMTokenList = js.native
  var innerHTML: String = js.native
  var outerHTML: String = js.native
  var slot: String = js.native
  def attributes: NamedNodeMap = js.native
  def children: HTMLCollection[Element] = js.native
  def childElementCount: Int = js.native
  def firstElementChild: Element = js.native
  def lastElementChild: Element = js.native
  def previousElementSibling: Element = js.native
  def nextElementSibling: Element = js.native
  var scrollTop: Double = js.native
  var scrollLeft: Double = js.native
  def scrollWidth: Int = js.native
  def scrollHeight: Int = js.native
  def clientTop: Int = js.native
  def clientLeft: Int = js.native
  def clientWidth: Int = js.native
  def clientHeight: Int = js.native
  def getAttribute(name: String): String = js.native
  def setAttribute(name: String, value: String): Unit = js.native
  def removeAttribute(name: String): Unit = js.native
  def hasAttribute(name: String): Boolean = js.native
  def hasAttributes(): Boolean = js.native
  def getAttributeNames(): js.Array[String] = js.native
  def toggleAttribute(name: String, force: Boolean = js.native): Boolean = js.native
  def getBoundingClientRect(): DOMRect = js.native
  def getClientRects(): DOMList[DOMRect] = js.native
  def querySelector(selectors: String): Element = js.native
  def querySelectorAll(selectors: String): NodeList[Element] = js.native
  def getElementsByTagName(name: String): HTMLCollection[Element] = js.native
  def getElementsByClassName(classNames: String): HTMLCollection[Element] = js.native
  def closest(selector: String): Element = js.native
  def matches(selector: String): Boolean = js.native
  def remove(): Unit = js.native
  def append(nodes: (Node | String)*): Unit = js.native
  def prepend(nodes: (Node | String)*): Unit = js.native
  def before(nodes: (Node | String)*): Unit = js.native
  def after(nodes: (Node | String)*): Unit = js.native
  def replaceWith(nodes: (Node | String)*): Unit = js.native
  def replaceChildren(nodes: (Node | String)*): Unit = js.native
  def insertAdjacentHTML(where: String, html: String): Unit = js.native
  def insertAdjacentElement(position: String, element: Element): Element = js.native
  def insertAdjacentText(position: String, text: String): Unit = js.native
  def scrollIntoView(top: Boolean | ScrollIntoViewOptions = js.native): Unit = js.native
  def scrollTo(x: Double | Int | ScrollToOptions, y: Double = js.native): Unit = js.native
  def scrollBy(x: Double | Int | ScrollToOptions, y: Double = js.native): Unit = js.native
  def requestFullscreen(options: js.Object = js.native): js.Promise[Unit] = js.native
  def requestPointerLock(): Unit = js.native
  def hasPointerCapture(pointerId: Double): Boolean = js.native
  def setPointerCapture(pointerId: Double): Unit = js.native
  def releasePointerCapture(pointerId: Double): Unit = js.native
  def shadowRoot: ShadowRoot = js.native
  def attachShadow(init: js.Object): ShadowRoot = js.native
  def getAnimations(): js.Array[js.Object] = js.native

trait ScrollIntoViewOptions extends js.Object:
  var behavior: js.UndefOr[String] = js.undefined
  var block: js.UndefOr[String] = js.undefined
  var inline: js.UndefOr[String] = js.undefined

trait ScrollToOptions extends js.Object:
  var top: js.UndefOr[Double] = js.undefined
  var left: js.UndefOr[Double] = js.undefined
  var behavior: js.UndefOr[String] = js.undefined

@js.native
@JSGlobal("NamedNodeMap")
class NamedNodeMap extends DOMList[Attr]:
  def item(index: Int): Attr = js.native
  def getNamedItem(name: String): Attr = js.native

@js.native
@JSGlobal("Attr")
class Attr extends Node:
  def name: String = js.native
  var value: String = js.native
  def ownerElement: Element = js.native

@js.native
@JSGlobal("CharacterData")
class CharacterData extends Node:
  var data: String = js.native
  def length: Int = js.native

@js.native
@JSGlobal("Text")
class Text(data: String = js.native) extends CharacterData:
  def wholeText: String = js.native
  def splitText(offset: Int): Text = js.native

@js.native
@JSGlobal("Comment")
class Comment(data: String = js.native) extends CharacterData

@js.native
@JSGlobal("DocumentFragment")
class DocumentFragment() extends Node:
  def children: HTMLCollection[Element] = js.native
  def querySelector(selectors: String): Element = js.native
  def querySelectorAll(selectors: String): NodeList[Element] = js.native
  def getElementById(elementId: String): Element = js.native

@js.native
@JSGlobal("ShadowRoot")
class ShadowRoot extends DocumentFragment:
  def host: Element = js.native
  def mode: String = js.native
  var innerHTML: String = js.native
  def activeElement: Element = js.native

@js.native
@JSGlobal("Document")
class Document extends Node:
  def documentElement: Element = js.native
  def doctype: Node = js.native
  def documentURI: String = js.native
  def characterSet: String = js.native
  def children: HTMLCollection[Element] = js.native
  def childElementCount: Int = js.native
  def firstElementChild: Element = js.native
  def lastElementChild: Element = js.native
  def getElementById(elementId: String): Element = js.native
  def getElementsByTagName(name: String): HTMLCollection[Element] = js.native
  def getElementsByClassName(classNames: String): HTMLCollection[Element] = js.native
  def getElementsByName(elementName: String): NodeList[Node] = js.native
  def querySelector(selectors: String): Element = js.native
  def querySelectorAll(selectors: String): NodeList[Element] = js.native
  def elementFromPoint(x: Double, y: Double): Element = js.native
  def elementsFromPoint(x: Double, y: Double): js.Array[Element] = js.native
  def createElement(tagName: String, options: js.Object = js.native): Element = js.native
  def createElementNS(namespaceURI: String, qualifiedName: String, options: js.Object = js.native): Element = js.native
  def createTextNode(data: String): Text = js.native
  def createComment(data: String): Comment = js.native
  def createDocumentFragment(): DocumentFragment = js.native
  def createAttribute(name: String): Attr = js.native
  def createRange(): Range = js.native
  def createEvent(eventInterface: String): Event = js.native
  def importNode(importedNode: Node, deep: Boolean = js.native): Node = js.native
  def adoptNode(source: Node): Node = js.native
  def hidden: Boolean = js.native
  def visibilityState: String = js.native
  def fullscreenElement: Element = js.native
  def fullscreenEnabled: Boolean = js.native
  def exitFullscreen(): js.Promise[Unit] = js.native
  def pointerLockElement: Element = js.native
  def exitPointerLock(): Unit = js.native
  var onfullscreenchange: js.Function1[Event, Any] = js.native
  var onfullscreenerror: js.Function1[Event, Any] = js.native
  var onvisibilitychange: js.Function1[Event, Any] = js.native

@js.native
@JSGlobal("HTMLDocument")
class HTMLDocument extends Document:
  var title: String = js.native
  var domain: String = js.native
  var cookie: String = js.native
  var dir: String = js.native
  var designMode: String = js.native
  def location: Location = js.native
  def URL: String = js.native
  def referrer: String = js.native
  def readyState: String = js.native
  def compatMode: String = js.native
  def defaultView: Window = js.native
  def head: HTMLHeadElement = js.native
  var body: HTMLElement = js.native
  def activeElement: Element = js.native
  def forms: HTMLCollection[Element] = js.native
  def images: HTMLCollection[Element] = js.native
  def links: HTMLCollection[Element] = js.native
  def scripts: HTMLCollection[Element] = js.native
  def execCommand(commandId: String, showUI: Boolean = js.native, value: js.Any = js.native): Boolean = js.native
  def queryCommandSupported(commandId: String): Boolean = js.native
  def queryCommandEnabled(commandId: String): Boolean = js.native
  def hasFocus(): Boolean = js.native
  def getSelection(): Selection = js.native
  def write(content: String*): Unit = js.native
  def open(): js.Dynamic = js.native
  def close(): Unit = js.native
  var onload: js.Function1[Event, Any] = js.native
  var onreadystatechange: js.Function1[Event, Any] = js.native
  var onselectionchange: js.Function1[Event, Any] = js.native
  var onkeydown: js.Function1[KeyboardEvent, Any] = js.native
  var onkeyup: js.Function1[KeyboardEvent, Any] = js.native
  var onclick: js.Function1[MouseEvent, Any] = js.native
  var onmousedown: js.Function1[MouseEvent, Any] = js.native
  var onmousemove: js.Function1[MouseEvent, Any] = js.native
  var onmouseup: js.Function1[MouseEvent, Any] = js.native
  var onscroll: js.Function1[UIEvent, Any] = js.native
  var onerror: js.Function1[ErrorEvent, Any] = js.native

@js.native
@JSGlobal("DOMRectReadOnly")
class DOMRectReadOnly extends js.Object:
  def x: Double = js.native
  def y: Double = js.native
  def width: Double = js.native
  def height: Double = js.native
  def top: Double = js.native
  def right: Double = js.native
  def bottom: Double = js.native
  def left: Double = js.native

@js.native
@JSGlobal("DOMRect")
class DOMRect(xArg: Double = js.native, yArg: Double = js.native, widthArg: Double = js.native, heightArg: Double = js.native) extends DOMRectReadOnly

@js.native
@JSGlobal("Selection")
class Selection extends js.Object:
  def anchorNode: Node = js.native
  def anchorOffset: Int = js.native
  def focusNode: Node = js.native
  def focusOffset: Int = js.native
  def isCollapsed: Boolean = js.native
  def rangeCount: Int = js.native
  def `type`: String = js.native
  def getRangeAt(index: Int): Range = js.native
  def addRange(range: Range): Unit = js.native
  def removeRange(range: Range): Unit = js.native
  def removeAllRanges(): Unit = js.native
  def empty(): Unit = js.native
  def collapse(node: Node, offset: Int = js.native): Unit = js.native
  def collapseToStart(): Unit = js.native
  def collapseToEnd(): Unit = js.native
  def extend(node: Node, offset: Int = js.native): Unit = js.native
  def selectAllChildren(node: Node): Unit = js.native
  def deleteFromDocument(): Unit = js.native
  def containsNode(node: Node, allowPartialContainment: Boolean = js.native): Boolean = js.native

@js.native
@JSGlobal("Range")
class Range() extends js.Object:
  def startContainer: Node = js.native
  def startOffset: Int = js.native
  def endContainer: Node = js.native
  def endOffset: Int = js.native
  def collapsed: Boolean = js.native
  def commonAncestorContainer: Node = js.native
  def setStart(refNode: Node, offset: Int): Unit = js.native
  def setEnd(refNode: Node, offset: Int): Unit = js.native
  def setStartBefore(refNode: Node): Unit = js.native
  def setStartAfter(refNode: Node): Unit = js.native
  def setEndBefore(refNode: Node): Unit = js.native
  def setEndAfter(refNode: Node): Unit = js.native
  def selectNode(refNode: Node): Unit = js.native
  def selectNodeContents(refNode: Node): Unit = js.native
  def collapse(toStart: Boolean = js.native): Unit = js.native
  def cloneContents(): DocumentFragment = js.native
  def deleteContents(): Unit = js.native
  def extractContents(): DocumentFragment = js.native
  def insertNode(newNode: Node): Unit = js.native
  def surroundContents(newParent: Node): Unit = js.native
  def cloneRange(): Range = js.native
  def detach(): Unit = js.native
  def getBoundingClientRect(): DOMRect = js.native
  def getClientRects(): DOMList[DOMRect] = js.native
  def createContextualFragment(fragment: String): DocumentFragment = js.native

@js.native
@JSGlobal("CSSStyleDeclaration")
class CSSStyleDeclaration extends js.Object:
  var cssText: String = js.native
  def length: Int = js.native
  def item(index: Int): String = js.native
  def getPropertyValue(propertyName: String): String = js.native
  def getPropertyPriority(propertyName: String): String = js.native
  def setProperty(propertyName: String, value: String, priority: String = js.native): Unit = js.native
  def removeProperty(propertyName: String): String = js.native
  var alignItems: String = js.native
  var alignSelf: String = js.native
  var animation: String = js.native
  var background: String = js.native
  var backgroundColor: String = js.native
  var backgroundImage: String = js.native
  var backgroundPosition: String = js.native
  var backgroundRepeat: String = js.native
  var backgroundSize: String = js.native
  var border: String = js.native
  var borderBottom: String = js.native
  var borderColor: String = js.native
  var borderLeft: String = js.native
  var borderRadius: String = js.native
  var borderRight: String = js.native
  var borderStyle: String = js.native
  var borderTop: String = js.native
  var borderWidth: String = js.native
  var bottom: String = js.native
  var boxShadow: String = js.native
  var boxSizing: String = js.native
  var clear: String = js.native
  var clip: String = js.native
  var clipPath: String = js.native
  var color: String = js.native
  var columnGap: String = js.native
  var content: String = js.native
  var cssFloat: String = js.native
  var cursor: String = js.native
  var direction: String = js.native
  var display: String = js.native
  var fill: String = js.native
  var filter: String = js.native
  var flex: String = js.native
  var flexDirection: String = js.native
  var flexGrow: String = js.native
  var flexShrink: String = js.native
  var flexWrap: String = js.native
  var font: String = js.native
  var fontFamily: String = js.native
  var fontSize: String = js.native
  var fontStyle: String = js.native
  var fontWeight: String = js.native
  var gap: String = js.native
  var gridTemplateColumns: String = js.native
  var gridTemplateRows: String = js.native
  var height: String = js.native
  var inset: String = js.native
  var justifyContent: String = js.native
  var left: String = js.native
  var letterSpacing: String = js.native
  var lineHeight: String = js.native
  var listStyle: String = js.native
  var margin: String = js.native
  var marginBottom: String = js.native
  var marginLeft: String = js.native
  var marginRight: String = js.native
  var marginTop: String = js.native
  var maxHeight: String = js.native
  var maxWidth: String = js.native
  var minHeight: String = js.native
  var minWidth: String = js.native
  var opacity: String = js.native
  var outline: String = js.native
  var overflow: String = js.native
  var overflowX: String = js.native
  var overflowY: String = js.native
  var padding: String = js.native
  var paddingBottom: String = js.native
  var paddingLeft: String = js.native
  var paddingRight: String = js.native
  var paddingTop: String = js.native
  var perspective: String = js.native
  var pointerEvents: String = js.native
  var position: String = js.native
  var resize: String = js.native
  var right: String = js.native
  var stroke: String = js.native
  var strokeWidth: String = js.native
  var textAlign: String = js.native
  var textDecoration: String = js.native
  var textOverflow: String = js.native
  var textShadow: String = js.native
  var textTransform: String = js.native
  var top: String = js.native
  var touchAction: String = js.native
  var transform: String = js.native
  var transformOrigin: String = js.native
  var transition: String = js.native
  var transitionDelay: String = js.native
  var transitionDuration: String = js.native
  var transitionProperty: String = js.native
  var userSelect: String = js.native
  var verticalAlign: String = js.native
  var visibility: String = js.native
  var whiteSpace: String = js.native
  var width: String = js.native
  var willChange: String = js.native
  var wordBreak: String = js.native
  var wordWrap: String = js.native
  var zIndex: String = js.native
