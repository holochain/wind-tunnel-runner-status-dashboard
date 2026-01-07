function handleHostnameFormSubmit(event) {
    event.preventDefault();
    window.location.href = '/hostname/' + document.getElementById('hostname').value;
}