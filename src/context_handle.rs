use crate::GGeom;
use c_vec::CVec;
use enums::{ByteOrder, Dimensions};
use error::{Error, GResult};
use ffi::*;
use std::sync::{Arc, Mutex};

pub struct GContextHandle {
    ptr: GEOSContextHandle_t,
    // boxed for stable address
    messages: Arc<Mutex<Messages>>,
    notice_closure: Box<dyn Fn(&str)>,
    error_closure: Box<dyn Fn(&str)>,
}

#[derive(Default)]
struct Messages {
    last_error: Option<String>,
    last_notice: Option<String>,
}

impl GContextHandle {
    /// Creates a new `GContextHandle`.
    ///
    /// # Example
    ///
    /// ```
    /// use geos::GContextHandle;
    ///
    /// let context_handle = GContextHandle::new().expect("invalid init");
    /// ```
    pub fn new() -> GResult<Self> {
        let ptr = unsafe { GEOS_init_r() };
        if ptr.is_null() {
            return Err(Error::GenericError("GEOS_init_r failed".to_owned()))
        }
        let messages = Arc::new(Mutex::new(Messages::default()));
        let notice_closure = {
            let messages = messages.clone();
            move |s: &str| messages.lock().unwrap().last_notice = Some(s.to_string())
        };
        let error_closure = {
            let messages = messages.clone();
            move |s: &str| messages.lock().unwrap().last_error = Some(s.to_string())
        };
        let res = GContextHandle {
            ptr,
            messages,
            notice_closure: Box::new(notice_closure),
            error_closure: Box::new(error_closure),
        };
        // TODO: set handlers...
        Ok(res)
    }

    pub fn take_last_notice(&self) -> Option<String> {
        self.messages.lock().unwrap().last_notice.take()
    }

    fn get_error(&self) -> String {
        self.messages.lock().unwrap().last_error.take().unwrap_or("unknown error".to_string())
    }

    /// Gets WKB output dimensions.
    ///
    /// # Example
    ///
    /// ```
    /// use geos::{GContextHandle, Dimensions};
    ///
    /// let context_handle = GContextHandle::new().expect("invalid init");
    ///
    /// context_handle.set_wkb_output_dimensions(Dimensions::TwoD);
    /// assert!(context_handle.get_wkb_output_dimensions() == Dimensions::TwoD);
    /// ```
    pub fn get_wkb_output_dimensions(&self) -> Dimensions {
        Dimensions::from(unsafe { GEOS_getWKBOutputDims_r(self.ptr) })
    }

    /// Sets WKB output dimensions.
    ///
    /// # Example
    ///
    /// ```
    /// use geos::{GContextHandle, Dimensions};
    ///
    /// let context_handle = GContextHandle::new().expect("invalid init");
    ///
    /// context_handle.set_wkb_output_dimensions(Dimensions::TwoD);
    /// assert!(context_handle.get_wkb_output_dimensions() == Dimensions::TwoD);
    /// ```
    pub fn set_wkb_output_dimensions(&self, dimensions: Dimensions) -> Dimensions {
        Dimensions::from(unsafe { GEOS_setWKBOutputDims_r(self.ptr, dimensions.into()) })
    }

    /// Gets WKB byte order.
    ///
    /// # Example
    ///
    /// ```
    /// use geos::{GContextHandle, ByteOrder};
    ///
    /// let context_handle = GContextHandle::new().expect("invalid init");
    ///
    /// context_handle.set_wkb_byte_order(ByteOrder::LittleEndian);
    /// assert!(context_handle.get_wkb_byte_order() == ByteOrder::LittleEndian);
    /// ```
    pub fn get_wkb_byte_order(&self) -> ByteOrder {
        ByteOrder::from(unsafe { GEOS_getWKBByteOrder_r(self.ptr) })
    }

    /// Sets WKB byte order.
    ///
    /// # Example
    ///
    /// ```
    /// use geos::{GContextHandle, ByteOrder};
    ///
    /// let context_handle = GContextHandle::new().expect("invalid init");
    ///
    /// context_handle.set_wkb_byte_order(ByteOrder::LittleEndian);
    /// assert!(context_handle.get_wkb_byte_order() == ByteOrder::LittleEndian);
    /// ```
    pub fn set_wkb_byte_order(&self, byte_order: ByteOrder) -> ByteOrder {
        ByteOrder::from(unsafe { GEOS_setWKBByteOrder_r(self.ptr, byte_order.into()) })
    }

    /// Convert [`GGeom`] from WKB format.
    ///
    /// # Example
    ///
    /// ```
    /// use geos::{GContextHandle, GGeom};
    ///
    /// let context_handle = GContextHandle::new().expect("invalid init");
    /// let point_geom = GGeom::new("POINT (2.5 2.5)").expect("Invalid geometry");
    /// let wkb_buf = context_handle.geom_to_wkb_buf(&point_geom)
    ///                             .expect("conversion to WKB failed");
    /// let new_geom = context_handle.geom_from_wkb_buf(wkb_buf.as_ref())
    ///                              .expect("conversion from WKB failed");
    /// assert!(point_geom.equals(&new_geom) == Ok(true));
    /// ```
    pub fn geom_from_wkb_buf(&self, wkb: &[u8]) -> GResult<GGeom> {
        unsafe { GGeom::new_from_raw(GEOSGeomFromWKB_buf_r(self.ptr, wkb.as_ptr(), wkb.len())) }
    }

    /// Convert [`GGeom`] to WKB format.
    ///
    /// # Example
    ///
    /// ```
    /// use geos::{GContextHandle, GGeom};
    ///
    /// let context_handle = GContextHandle::new().expect("invalid init");
    /// let point_geom = GGeom::new("POINT (2.5 2.5)").expect("Invalid geometry");
    /// let wkb_buf = context_handle.geom_to_wkb_buf(&point_geom)
    ///                             .expect("conversion to WKB failed");
    /// let new_geom = context_handle.geom_from_wkb_buf(wkb_buf.as_ref())
    ///                              .expect("conversion from WKB failed");
    /// assert!(point_geom.equals(&new_geom) == Ok(true));
    /// ```
    pub fn geom_to_wkb_buf(&self, g: &GGeom) -> GResult<CVec<u8>> {
        let mut size = 0;
        unsafe {
            let ptr = GEOSGeomToWKB_buf_r(self.ptr, g.as_raw(), &mut size);
            if ptr.is_null() {
                Err(Error::GenericError(self.get_error()))
            } else {
                Ok(CVec::new(ptr, size as _))
            }
        }
    }

    /// Convert [`GGeom`] from HEX format.
    ///
    /// # Example
    ///
    /// ```
    /// use geos::{GContextHandle, GGeom};
    ///
    /// let context_handle = GContextHandle::new().expect("invalid init");
    /// let point_geom = GGeom::new("POINT (2.5 2.5)").expect("Invalid geometry");
    /// let wkb_buf = context_handle.geom_to_hex_buf(&point_geom)
    ///                             .expect("conversion to HEX failed");
    /// let new_geom = context_handle.geom_from_hex_buf(wkb_buf.as_ref())
    ///                              .expect("conversion from HEX failed");
    /// assert!(point_geom.equals(&new_geom) == Ok(true));
    /// ```
    pub fn geom_from_hex_buf(&self, hex: &[u8]) -> GResult<GGeom> {
        unsafe { GGeom::new_from_raw(GEOSGeomFromHEX_buf_r(self.ptr, hex.as_ptr(), hex.len())) }
    }

    /// Convert [`GGeom`] to HEX format.
    ///
    /// # Example
    ///
    /// ```
    /// use geos::{GContextHandle, GGeom};
    ///
    /// let context_handle = GContextHandle::new().expect("invalid init");
    /// let point_geom = GGeom::new("POINT (2.5 2.5)").expect("Invalid geometry");
    /// let wkb_buf = context_handle.geom_to_hex_buf(&point_geom)
    ///                             .expect("conversion to HEX failed");
    /// let new_geom = context_handle.geom_from_hex_buf(wkb_buf.as_ref())
    ///                              .expect("conversion from HEX failed");
    /// assert!(point_geom.equals(&new_geom) == Ok(true));
    /// ```
    pub fn geom_to_hex_buf(&self, g: &GGeom) -> Option<CVec<u8>> {
        let mut size = 0;
        unsafe {
            let ptr = GEOSGeomToHEX_buf_r(self.ptr, g.as_raw(), &mut size);
            if ptr.is_null() {
                None
            } else {
                Some(CVec::new(ptr, size as _))
            }
        }
    }
}

impl Drop for GContextHandle {
    fn drop<'a>(&'a mut self) {
        unsafe { GEOS_finish_r(self.ptr) };

        // TODO: cleanup handlers
    }
}
