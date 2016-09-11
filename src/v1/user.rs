use std::io::Read;

use hyper::method::Method;
use hyper::header::{Headers, Authorization};
use rustc_serialize::json;

use chrono::NaiveDate;

use utils::Address;
use dto::{FromDTO, UserDTO, ScopeDTO as Scope, AuthenticationCodeDTO, ResponseDTO, UpdateUserDTO};

use super::{Client, VoidDTO};

use error::{Result, Error};
use super::types::User;
use super::oauth::AccessToken;

/// User methods for the client.
///
/// This are the user getters, setters and creators for the client.
impl Client {
    /// Resends the email confirmation
    pub fn resend_email_confirmation(&self, access_token: &AccessToken) -> Result<()> {
        let mut user_id = None;
        for scope in access_token.scopes() {
            match scope {
                &Scope::User(id) => user_id = Some(id),
                _ => {}
            }
        }
        if user_id.is_some() && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let _ = try!(self.send_request(Method::Get,
                                           format!("{}resend_email_confirmation", self.url),
                                           headers,
                                           None::<&VoidDTO>));
            Ok(())
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired user token")))
        }
    }

    /// Get the user
    pub fn get_user(&self, access_token: &AccessToken, user_id: u64) -> Result<User> {
        if access_token.scopes().any(|s| match s {
            &Scope::User(u_id) => u_id == user_id,
            &Scope::Admin => true,
            _ => false,
        }) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let mut response = try!(self.send_request(Method::Get,
                                                      format!("{}user/{}", self.url, user_id),
                                                      headers,
                                                      None::<&VoidDTO>));
            let mut response_str = String::new();
            let _ = try!(response.read_to_string(&mut response_str));
            Ok(try!(User::from_dto(try!(json::decode::<UserDTO>(&response_str)))))
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired admin or user \
                                               token, and in the case of a user token, the ID \
                                               in the token must match the given ID")))
        }
    }

    /// Gets the logged in users info
    pub fn get_me(&self, access_token: &AccessToken) -> Result<User> {
        let mut user_id = 0;
        if access_token.scopes().any(|s| match s {
            &Scope::User(id) => {
                user_id = id;
                true
            }
            _ => false,
        }) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let mut response = try!(self.send_request(Method::Get,
                                                      format!("{}user/{}", self.url, user_id),
                                                      headers,
                                                      None::<&VoidDTO>));
            let mut response_str = String::new();
            let _ = try!(response.read_to_string(&mut response_str));
            Ok(try!(User::from_dto(try!(json::decode::<UserDTO>(&response_str)))))
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired user token")))
        }
    }

    /// Gets all users.
    pub fn get_all_users(&self, access_token: &AccessToken) -> Result<Vec<User>> {
        if access_token.scopes().any(|s| s == &Scope::Admin) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let mut response = try!(self.send_request(Method::Get,
                                                      format!("{}all_users", self.url),
                                                      headers,
                                                      None::<&VoidDTO>));
            let mut response_str = String::new();
            let _ = try!(response.read_to_string(&mut response_str));
            let dto_users: Vec<UserDTO> = try!(json::decode(&response_str));
            Ok(dto_users.into_iter()
                .filter_map(|u| match User::from_dto(u) {
                    Ok(u) => Some(u),
                    Err(_) => None,
                })
                .collect())
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired admin token")))
        }
    }

    /// Deletes the given user.
    pub fn delete_user(&self, access_token: &AccessToken, user_id: u64) -> Result<()> {
        if access_token.scopes().any(|s| s == &Scope::Admin) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let _ = try!(self.send_request(Method::Delete,
                                           format!("{}user/{}", self.url, user_id),
                                           headers,
                                           None::<&VoidDTO>));
            Ok(())
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired admin token")))
        }
    }

    // TODO update user

    /// Generates a new authenticator code, and returns the URL.
    pub fn generate_authenticator_code(&self, access_token: &AccessToken) -> Result<String> {
        if access_token.scopes().any(|s| match s {
            &Scope::User(_) => true,
            _ => false,
        }) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let mut response = try!(self.send_request(Method::Get,
                                                      format!("{}generate_authenticator_code",
                                                              self.url),
                                                      headers,
                                                      None::<&VoidDTO>));
            let mut response_str = String::new();
            let _ = try!(response.read_to_string(&mut response_str));
            Ok(try!(json::decode::<ResponseDTO>(&response_str)).message)
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired user token")))
        }
    }

    /// Authenticates the user with 2FA
    pub fn authenticate(&self, access_token: &AccessToken, code: u32) -> Result<()> {
        if access_token.scopes().any(|s| match s {
            &Scope::User(_) => true,
            _ => false,
        }) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let dto = AuthenticationCodeDTO { code: code };
            let _ = try!(self.send_request(Method::Post,
                                           format!("{}authenticate", self.url),
                                           headers,
                                           Some(&dto)));
            Ok(())
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired user token")))
        }
    }


    /// Sets the users username
    pub fn set_username<S: AsRef<str>>(&self,
                                       access_token: &AccessToken,
                                       user_id: u64,
                                       username: S)
                                       -> Result<()> {
        if access_token.scopes().any(|s| match s {
            &Scope::User(u_id) => u_id == user_id,
            &Scope::Admin => true,
            _ => false,
        }) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let dto = UpdateUserDTO {
                new_username: Some(String::from(username.as_ref())),
                new_email: None,
                new_first: None,
                new_last: None,
                old_password: None,
                new_password: None,
                new_phone: None,
                new_birthday: None,
                new_image: None,
                new_address: None,
            };
            let _ = try!(self.send_request(Method::Post,
                                           format!("{}update_user/{}", self.url, user_id),
                                           headers,
                                           Some(&dto)));
            Ok(())
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired admin or user \
                                               token, and in the case of a user token, the ID \
                                               in the token must match the given ID")))
        }
    }

    /// Sets the users phone
    pub fn set_phone<S: AsRef<str>>(&self,
                                    access_token: &AccessToken,
                                    user_id: u64,
                                    phone: S)
                                    -> Result<()> {
        if access_token.scopes().any(|s| match s {
            &Scope::User(u_id) => u_id == user_id,
            &Scope::Admin => true,
            _ => false,
        }) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let dto = UpdateUserDTO {
                new_username: None,
                new_email: None,
                new_first: None,
                new_last: None,
                old_password: None,
                new_password: None,
                new_phone: Some(String::from(phone.as_ref())),
                new_birthday: None,
                new_image: None,
                new_address: None,
            };
            let _ = try!(self.send_request(Method::Post,
                                           format!("{}update_user/{}", self.url, user_id),
                                           headers,
                                           Some(&dto)));
            Ok(())
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired admin or user \
                                               token, and in the case of a user token, the ID \
                                               in the token must match the given ID")))
        }
    }

    /// Sets the users birthday
    pub fn set_birthday<S: AsRef<str>>(&self,
                                       access_token: &AccessToken,
                                       user_id: u64,
                                       birthday: NaiveDate)
                                       -> Result<()> {
        if access_token.scopes().any(|s| match s {
            &Scope::User(u_id) => u_id == user_id,
            &Scope::Admin => true,
            _ => false,
        }) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let dto = UpdateUserDTO {
                new_username: None,
                new_email: None,
                new_first: None,
                new_last: None,
                old_password: None,
                new_password: None,
                new_phone: None,
                new_birthday: Some(birthday),
                new_image: None,
                new_address: None,
            };
            let _ = try!(self.send_request(Method::Post,
                                           format!("{}update_user/{}", self.url, user_id),
                                           headers,
                                           Some(&dto)));
            Ok(())
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired admin or user \
                                               token, and in the case of a user token, the ID \
                                               in the token must match the given ID")))
        }
    }

    /// Sets the users first and last name
    pub fn set_name<S: AsRef<str>>(&self,
                                   access_token: &AccessToken,
                                   user_id: u64,
                                   first: S,
                                   last: S)
                                   -> Result<()> {
        if access_token.scopes().any(|s| match s {
            &Scope::User(u_id) => u_id == user_id,
            &Scope::Admin => true,
            _ => false,
        }) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let dto = UpdateUserDTO {
                new_username: None,
                new_email: None,
                new_first: Some(String::from(first.as_ref())),
                new_last: Some(String::from(last.as_ref())),
                old_password: None,
                new_password: None,
                new_phone: None,
                new_birthday: None,
                new_image: None,
                new_address: None,
            };
            let _ = try!(self.send_request(Method::Post,
                                           format!("{}update_user/{}", self.url, user_id),
                                           headers,
                                           Some(&dto)));
            Ok(())
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired admin or user \
                                               token, and in the case of a user token, the ID \
                                               in the token must match the given ID")))
        }
    }

    /// Sets the users email
    pub fn set_email<S: AsRef<str>>(&self,
                                    access_token: &AccessToken,
                                    user_id: u64,
                                    email: S)
                                    -> Result<()> {
        if access_token.scopes().any(|s| match s {
            &Scope::User(u_id) => u_id == user_id,
            &Scope::Admin => true,
            _ => false,
        }) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let dto = UpdateUserDTO {
                new_username: None,
                new_email: Some(String::from(email.as_ref())),
                new_first: None,
                new_last: None,
                old_password: None,
                new_password: None,
                new_phone: None,
                new_birthday: None,
                new_image: None,
                new_address: None,
            };
            let _ = try!(self.send_request(Method::Post,
                                           format!("{}update_user/{}", self.url, user_id),
                                           headers,
                                           Some(&dto)));
            Ok(())
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired admin or user \
                                               token, and in the case of a user token, the ID \
                                               in the token must match the given ID")))
        }
    }

    /// Sets the users profile picture to the given URL
    pub fn set_image<S: AsRef<str>>(&self,
                                    access_token: &AccessToken,
                                    user_id: u64,
                                    image_url: S)
                                    -> Result<()> {
        if access_token.scopes().any(|s| match s {
            &Scope::User(u_id) => u_id == user_id,
            &Scope::Admin => true,
            _ => false,
        }) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let dto = UpdateUserDTO {
                new_username: None,
                new_email: None,
                new_first: None,
                new_last: None,
                old_password: None,
                new_password: None,
                new_phone: None,
                new_birthday: None,
                new_image: Some(String::from(image_url.as_ref())),
                new_address: None,
            };
            let _ = try!(self.send_request(Method::Post,
                                           format!("{}update_user/{}", self.url, user_id),
                                           headers,
                                           Some(&dto)));
            Ok(())
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired admin or user \
                                               token, and in the case of a user token, the ID \
                                               in the token must match the given ID")))
        }
    }

    /// Sets the users address
    pub fn set_address<S: AsRef<str>>(&self,
                                      access_token: &AccessToken,
                                      user_id: u64,
                                      address: Address)
                                      -> Result<()> {
        if access_token.scopes().any(|s| match s {
            &Scope::User(u_id) => u_id == user_id,
            &Scope::Admin => true,
            _ => false,
        }) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let dto = UpdateUserDTO {
                new_username: None,
                new_email: None,
                new_first: None,
                new_last: None,
                old_password: None,
                new_password: None,
                new_phone: None,
                new_birthday: None,
                new_image: None,
                new_address: Some(address),
            };
            let _ = try!(self.send_request(Method::Post,
                                           format!("{}update_user/{}", self.url, user_id),
                                           headers,
                                           Some(&dto)));
            Ok(())
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired admin or user \
                                               token, and in the case of a user token, the ID \
                                               in the token must match the given ID")))
        }
    }

    /// Sets the user password
    pub fn set_password<S: AsRef<str>>(&self,
                                       access_token: &AccessToken,
                                       old_password: S,
                                       new_password: S)
                                       -> Result<()> {
        let mut user_id = 0;
        if access_token.scopes().any(|s| match s {
            &Scope::User(id) => {
                user_id = id;
                true
            }
            _ => false,
        }) && !access_token.has_expired() {
            let mut headers = Headers::new();
            headers.set(Authorization(access_token.get_token()));
            let dto = UpdateUserDTO {
                new_username: None,
                new_email: None,
                new_first: None,
                new_last: None,
                old_password: Some(String::from(old_password.as_ref())),
                new_password: Some(String::from(new_password.as_ref())),
                new_phone: None,
                new_birthday: None,
                new_image: None,
                new_address: None,
            };
            let _ = try!(self.send_request(Method::Post,
                                           format!("{}update_user/{}", self.url, user_id),
                                           headers,
                                           Some(&dto)));
            Ok(())
        } else {
            Err(Error::Forbidden(String::from("the token must be an unexpired admin or user \
                                               token, and in the case of a user token, the ID \
                                               in the token must match the given ID")))
        }
    }
}