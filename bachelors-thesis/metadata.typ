#let format_strane = "a4"
#let naslov = "Имплементација погона за дводимензионалне видео игре користећи ЕКС архитектуру и програмски језик Rust"
#let autor = "Никола Огњеновић"

// На енглеском
#let naslov_eng = "Implementation of a game engine for two-dimensional video games using ECS architecture and the Rust programming language"
#let autor_eng = "Nikola Ognjenović"

#let indeks = "SV 51 / 2022"

// Име и презиме ментора
#let mentor = "Игор Дејановић"
// Звање: редовни професор, ванредни професор, доцент
#let mentor_zvanje = "редовни професор"

// Скинути коментаре са одговарајућих линија
#let studijski_program = "Софтверско инжењерство и информационе технологије"
#let stepen = "Основне академске студије"

#let godina = [#datetime.today().year()]

#let kljucne_reci = "видео игре, Rust, ECS архитектура, погон, рендеринг, перформансе"
#let apstrakt = [
     Овај завршни рад описује имплементацију погона за дводимензионалне видео игре,
     развијеног у програмском језику Rust. Главни циљ је постизање високих
     перформанси уз стриктно поштовање безбедносних гаранција које пружа Rust,
     посебно кроз избегавање `unsafe` кода. Рад представља имплементацију ЕКС
     (Entity Component System) архитектуре која омогућава модуларност и
     ефикасну обраду података. Истражују се различити приступи рендеровању
     уз коришћење WebGPU стандарда, уз анализу утицаја ECS дизајна на
     укупне перформансе погона.
]

// На енглеском
#let kljucne_reci_eng = "video games, Rust, ECS architecture, game engine, rendering, performance"
#let apstrakt_eng = [
     This thesis describes the implementation of a 2D game engine developed in the
     Rust programming language. The main objective is to achieve high performance
     while strictly adhering to the safety guarantees provided by Rust,
     particularly by avoiding `unsafe` code. The thesis presents the
     implementation of an Entity Component System (ECS) architecture that
     enables modularity and efficient data processing. Various rendering
     approaches using the WebGPU standard are explored, along with an analysis
     of the impact of ECS design on the overall performance of the engine.
]

// TODO: Текст задатка добијате од ментора. Заменити доле #lorem(100) са текстом задатка.
#let zadatak = [
     #lorem(100)
]

// TODO: Датум одбране и чланове комисије добијате од ментора
#let datum_odbrane = "01.01.2025"
#let komisija_predsednik = "Петар Петровић"
#let komisija_predsednik_zvanje = "ванредни професор"
#let komisija_clan = "Марко Марковић"
#let komisija_clan_zvanje = "доцент"

// На енглеском уписати чланове на латиници
#let komisija_predsednik_eng = "Petar Petrović"
#let komisija_clan_eng = "Marko Marković"
#let mentor_eng = "Igor Dejanović"


// Ово даље углавном не треба мењати.

#let zvanje_eng = (
     "редовни професор": "full professor",
     "ванредни професор": "assoc. professor",
     "доцент": "asist. professor",
)
#let komisija_predsednik_zvanje_eng = zvanje_eng.at(komisija_predsednik_zvanje)
#let komisija_clan_zvanje_eng = zvanje_eng.at(komisija_clan_zvanje)
#let mentor_zvanje_eng = zvanje_eng.at(mentor_zvanje)


#let vrsta_rada = if stepen == "Мастер академске студије" {
    "Дипломски - мастер рад"
} else {
    "Дипломски - бечелор рад"
}

#let oblast = "Електротехничко и рачунарско инжењерство"
#let oblast_eng = "Electrical and Computer Engineering"
#let disciplina = "Примењене рачунарске науке и информатика"
#let disciplina_eng = "Applied computer science and informatics"

#import "funkcije.typ": *
// Поглавља/страна/цитата/табела/слика/графика/прилога
#let fizicki_opis = physical()
