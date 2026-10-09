package meridian.web.component

import meridian.core.effect.{Eff, Ref, Task, UIO}
import meridian.core.optics.Lens
import meridian.web.vdom.Renderer

/** The application state, read by components through selectors and written through lenses;
  * every write invalidates the renderer, which re-renders the subscribed components. */
final class Store[S](initial: S):
  private val ref: Ref[S] = Ref.unsafeMake(initial)
  private var subscribers: Set[String] = Set.empty
  private var writes: Int = 0

  def current: S = ref.unsafeGet
  def get: UIO[S] = ref.get
  def set(s: S): UIO[Unit] = ref.set(s).flatMap(_ => publish)
  def modify(f: S => S): UIO[Unit] = ref.update(f).flatMap(_ => publish)
  def setL[T](lens: Lens[S, T])(value: T): UIO[Unit] = modify(lens.replace(value))
  def modL[T](lens: Lens[S, T])(f: T => T): UIO[Unit] = modify(lens.modify(f))
  def read[T](lens: Lens[S, T]): UIO[T] = get.map(lens.get)
  def subscribe(path: String): Subscription =
    subscribers += path
    new Subscription(path)
  def subscriberCount: Int = subscribers.size
  def writeCount: Int = writes
  private def publish: UIO[Unit] = Eff.succeed {
    writes += 1
    if subscribers.nonEmpty then Renderer.invalidate()
  }

final class Subscription(val path: String)

object Store:
  def apply[S](initial: S): Store[S] = new Store(initial)
