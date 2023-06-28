/// A trait for implementing a way to get the name of an `enum` variant.
pub trait VariantName {
	/// Gets the name of the `enum` variant.
	fn variant_name(&self) -> &'static str;
}
