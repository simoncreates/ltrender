use std::collections::HashMap;

use crate::draw::{AllSprites, DrawError, SpriteId, SpriteObjectType, UpdateInterval};
use ascii_assets::TerminalChar;
use common_stdx::{Point, Rect};

pub fn convert_rect_to_update_intervals(rect: Rect<u16>) -> HashMap<u16, Vec<UpdateInterval>> {
    let mut intervals: HashMap<u16, Vec<UpdateInterval>> = HashMap::new();
    let iv = (rect.p1.x, rect.p2.x);

    for y in rect.p1.y..rect.p2.y {
        let intv = UpdateInterval { interval: iv };
        intervals.entry(y).or_default().push(intv);
    }
    intervals
}

#[derive(Debug)]
pub struct BasicDraw {
    pub pos: Point<u16>,
    pub chr: TerminalChar,
}

pub trait Cloneable {
    fn clone_box(&self) -> Box<dyn Drawable>;
}

impl<T> Cloneable for T
where
    T: 'static + Drawable + Clone,
{
    fn clone_box(&self) -> Box<dyn Drawable> {
        Box::new(self.clone())
    }
}

pub trait Drawable: Cloneable + std::fmt::Debug + Send + Sync {
    /// Render this object into a list of terminal cells.
    ///
    /// The method should not perform any clipping, layering or buffer
    /// management; it only needs to return the raw `(pos, chr)` pairs that
    /// represent what would be drawn on screen.  Implementations are free
    /// to query `sprites` for additional information such as sprite size.
    ///
    /// ## Errors
    ///
    /// A `DrawError` is returned if something goes wrong while resolving the
    /// sprite data (e.g., missing sprite).
    ///
    /// ## Example
    ///
    /// ```rust
    /// let sprite = SpriteDrawable { position: Point::new(0, 0), sprite_id: 1 };
    /// let draws = sprite.draw(&all_sprites).unwrap();
    /// ```
    fn draw(&self, sprites: &AllSprites) -> Result<Vec<BasicDraw>, DrawError>;

    /// Return a map of update intervals keyed by row.
    ///
    /// Each entry tells the renderer which horizontal segments of a given
    /// row need to be refreshed because this object occupies that area.
    /// Implementations typically call `convert_rect_to_update_intervals` with
    /// the sprite's bounding rectangle.
    ///
    /// ## Example
    ///
    /// ```rust
    /// let intervals = sprite.bounding_iv(&all_sprites);
    /// ```
    fn bounding_iv(&self, sprites: &AllSprites) -> HashMap<u16, Vec<UpdateInterval>>;

    /// Return **a new** drawable that has been shifted by the given offset.
    ///
    /// The returned value is boxed as a trait object because the concrete
    /// type may differ between implementations.
    ///
    /// ## Parameters
    ///
    /// * `offset` – A point indicating how far to shift the current position.
    ///
    /// ## Returns
    ///
    /// A boxed trait object that implements `Drawable`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// // Assume we have a concrete implementation called `SpriteDrawable`
    /// let original = SpriteDrawable {
    ///     position: Point::new(0, 0),
    ///     sprite_id: 42,
    /// };
    /// let shifted = original.shifted(Point::new(10, 5));
    ///
    /// // The new drawable has the updated position.
    /// assert_eq!(shifted.position(), Point::new(10, 5));
    /// ```
    fn shifted(&self, offset: Point<u16>) -> Box<dyn Drawable>;
}

impl Clone for Box<dyn Drawable> {
    fn clone(&self) -> Box<dyn Drawable> {
        self.clone_box()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SpriteDrawable {
    pub position: Point<u16>,
    pub sprite_id: SpriteId,
}

impl Drawable for SpriteDrawable {
    fn draw(&self, sprites: &AllSprites) -> Result<Vec<BasicDraw>, DrawError> {
        let sprite = match sprites.get(&self.sprite_id) {
            Some(s) => s,
            None => return Err(DrawError::SpriteNotFound(self.sprite_id)),
        };

        match &sprite.info {
            SpriteObjectType::Sprite(content) => {
                let width = content.width as usize;
                let origin_x = self.position.x;
                let origin_y = self.position.y;

                let mut out = Vec::with_capacity(content.pixels.len());
                for (i, ch) in content.pixels.iter().enumerate() {
                    let dx = (i % width) as u16;
                    let dy = (i / width) as u16;

                    let abs_x = origin_x.saturating_add(dx);
                    let abs_y = origin_y.saturating_add(dy);

                    out.push(BasicDraw {
                        pos: Point { x: abs_x, y: abs_y },
                        chr: *ch,
                    });
                }
                Ok(out)
            }
            SpriteObjectType::SpriteVideo(content) => {
                unimplemented!("Sprite video rendering not implemented yet");
            }
        }
    }

    fn bounding_iv(&self, sprites: &AllSprites) -> HashMap<u16, Vec<UpdateInterval>> {
        let size = sprites
            .get(&self.sprite_id)
            .map(|s| s.size())
            .unwrap_or((0, 0, 0));
        convert_rect_to_update_intervals(Rect {
            p1: self.position,
            p2: Point {
                x: self.position.x + size.1 as u16,
                y: self.position.y + size.2 as u16,
            },
        })
    }

    fn shifted(&self, offset: Point<u16>) -> Box<dyn Drawable> {
        Box::new(SpriteDrawable {
            position: self.position + offset,
            sprite_id: self.sprite_id,
        })
    }
}
