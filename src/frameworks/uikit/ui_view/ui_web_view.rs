/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIWebView`.

use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::objc::{id, nil, objc_classes, ClassExports};
use crate::{msg, msg_super};
use std::borrow::Cow;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIWebView: UIView

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    msg_super![env; this initWithCoder:coder]
}

- (())setScalesPageToFit:(bool)_scales {
    // TODO
}
- (())setDelegate:(id)_delegate {
    // TODO
}
- (())loadRequest:(id)request { // NSURLRequest*
    let url_string: Cow<'_, str> = if request == nil {
        Cow::Borrowed("<nil request>")
    } else {
        let url = msg![env; request URL];
        if url == nil {
            Cow::Borrowed("<nil URL>")
        } else {
            let url_desc = msg![env; url description];
            if url_desc == nil {
                Cow::Borrowed("<nil URL description>")
            } else {
                to_rust_string(env, url_desc)
            }
        }
    };
    log!("TODO: [(UIWebView*) {:?} loadRequest:{:?} ({})]", this, request, url_string);
}

@end

};
