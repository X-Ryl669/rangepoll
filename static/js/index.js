function ignore() {}
function cancel(e) { e.preventDefault(); return false; }
function show(selector, state)
{
  var values = ['none', 'block', 'flex'];
  $(selector).css({display: values[state|0]});
}
function getCookie(cookiename) {
  var cookiestring = RegExp(cookiename+"=[^;]+").exec(document.cookie);
  return decodeURIComponent(!!cookiestring ? cookiestring.toString().replace(/^[^=]+./,"") : "");
}
function filter(a,k) { if (k in a) return a[k]; return ''; }
function sproutRandom(numChar) {
  var result = '', characters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789', l = numChar == undefined ? 10 : numChar;
  for ( var i = 0; i < l; i++ )
    result += characters.charAt(Math.floor(Math.random() * characters.length));
  return result;
}


function ajaxStarting(id) { show(id, 2); }
function ajaxEnding(id)   { show(id, false); }
function attr(e, name)    { if (name in e.attributes) return e.attributes[name].value; return undefined; }
function shouldLogin(url) { window.location.href = url; }
function formData(f) {
  return Array.from(f.elements).filter(function(i) { return i.name && (i.type != "radio" || i.checked == true) })
              .map(function(i) {
                    return encodeURIComponent(i.name) + '=' + encodeURIComponent(i.type == "checkbox" ? (i.checked ? 1 : 0) :
                    i.type == "password" ? forge_sha256(i.value + attr(i, "salt")) : i.value);
                  }).join('&');
}



function ajax(url, callback, error, options) {
  var xhr = new XMLHttpRequest();
  options = options || {};
  error = error || function(e) {};
  xhr.onreadystatechange = function() {
      if (xhr.readyState == 4) {
          if (xhr.status == 200)
              callback(xhr.responseText);
          else if (xhr.status == 401 && !options.skipAuth)
              shouldLogin(xhr.getResponseHeader('Location'));
          else error(xhr.responseText, options.errorArg);
          if (options.bannerId) ajaxEnding(options.bannerId);
      }
  }
  if (options.upload && typeof xhr.upload.onprogress != 'undefined') xhr.upload.onprogress = options.upload;
  xhr.ontimeout = xhr.onerror = function (e) { error(e, options.errorArg); }
  var method = options.method || "GET";
  xhr.open(method, url, true);
  xhr.timeout = options.timeout || 0;
  if (options.contentType) xhr.setRequestHeader('Content-Type', options.contentType);
  else if (method == "POST" && !options.upload) xhr.setRequestHeader('Content-Type', 'application/x-www-form-urlencoded;charset=UTF-8');
  if (options.bannerId) ajaxStarting(options.bannerId);
  if (options.progress) xhr.onprogress = function(e) {
    var total = e.total ? e.total : parseInt(e.target.getResponseHeader('Content-length'));
    $(options.progress).prop('max', total).prop('value', e.loaded);
    if (e.loaded >= total) $(options.progress).addClass('fadeOut');
  };
  xhr.send(options.formData || null);
}

function delegateEvent(element, event, descendentSelector, callback) {
  element.addEventListener(event, function(e){
    var elem = e.target.closest(descendentSelector);
    if(elem) callback(elem, e);
  }, false);
}

function showLogOut() {
    var user = getCookie("user");
    if (user.length) $('a.user').removeClass('h').text(`Hi, ${user}: Log out`);
}

function updateDialog(d)
{
    showLogOut();
    $('div.dialog').html(d);
}

function uponError(e)
{
    console.log(e);
}

ajax('poll_list', updateDialog, uponError, {});
window.onload = function() 
{
    showLogOut();
    delegateEvent($('div.dialog').a[0], 'click', '.loadVote', function(e)
    {
        ajax($(e).attr('data-href'), updateDialog, uponError, {});
    });
}