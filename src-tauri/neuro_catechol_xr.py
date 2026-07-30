import torch
import torch.nn as nn

class NeuroCatechol_XR:
    """
    Wirtualny lek Przedłużonego Uwalniania (XR) dla sieci neuronowych.
    Zaprojektowany specjalnie dla Kropelki (AI w VIBE).
    
    Skład (Farmakokinetyka): 
      - L-DOPA Nesterova (Dopamina Antycypacyjna)
      - Decoupled Aspirin (Celowana redukcja zapalenia w stylu AdamW)
      - GABA-Modulator (Hamowanie szoków wariancji błędu w stylu RMSProp)
    """
    
    def __init__(self, model: nn.Module, lr=0.001, 
                 anti_inflammatory_dose=1e-4, 
                 dopamine_anticipation=0.9, 
                 gaba_calming=0.999):
        self.model = model
        self.lr = lr
        
        # Dawkowanie leków
        self.weight_decay = anti_inflammatory_dose
        self.beta1 = dopamine_anticipation # Antycypacja dopaminowa (Momentum)
        self.beta2 = gaba_calming          # Uspokajacz GABA (Wariancja)
        self.epsilon = 1e-8                # Zabezpieczenie przed szokiem dzielenia przez 0
        
        # Rezerwa VRAM - "Maszyny podtrzymujące życie" podpinane przy wejściu w tryb uczenia
        # Będą one utrzymywane tylko w fazie Continuous Learning.
        self.dopamine_memory = {name: torch.zeros_like(param) for name, param in model.named_parameters()}
        self.stress_memory = {name: torch.zeros_like(param) for name, param in model.named_parameters()}
        
        self.time_step = 0 # Kliniczny czas trwania terapii na danym oddziale

    def inject_drug(self, loss: torch.Tensor):
        """
        Zabieg podania kroplówki. Wywoływany w każdej epoce uczenia (Backpropagation).
        Wymaga straty (bólu), by odpowiednio dozować ukojenie.
        """
        loss.backward()
        self.time_step += 1
        
        with torch.no_grad():
            for name, param in self.model.named_parameters():
                if param.grad is not None:
                    
                    grad = param.grad
                    
                    # --- 1. MODULACJA STRESU (GABA - Wariancja) ---
                    # Zapamiętywanie, jak często synapsa ulegała ostrym bólom.
                    self.stress_memory[name] = (self.beta2 * self.stress_memory[name]) + \
                                               ((1 - self.beta2) * (grad ** 2))
                    
                    # Bias correction - Korekta błędu na początku terapii
                    bias_correction_stress = 1 - self.beta2 ** self.time_step
                    calmed_stress = (self.stress_memory[name] / bias_correction_stress).sqrt() + self.epsilon
                    
                    # --- 2. DOPAMINA ANTYCYPACYJNA (L-DOPA z Nesterovem) ---
                    # Aktualizacja oczekiwań (Momentum)
                    self.dopamine_memory[name] = (self.beta1 * self.dopamine_memory[name]) + \
                                                 ((1 - self.beta1) * grad)
                                                 
                    bias_correction_dopamine = 1 - self.beta1 ** self.time_step
                    active_dopamine = self.dopamine_memory[name] / bias_correction_dopamine
                    
                    # Efekt Nesterova (Antycypacja - sieć patrzy tam, gdzie zmierza rzucając okiem w przyszłość)
                    lookahead_dopamine = (self.beta1 * active_dopamine) + ((1 - self.beta1) * grad / bias_correction_dopamine)

                    # --- 3. DZIAŁANIE PRZECIWZAPALNE (Decoupled Aspirin / L2) ---
                    # Aspiryna działa TERAZ BEZPOŚREDNIO na masę wagi, ułatwiając detoksykację i powstrzymując overfitting!
                    param -= self.lr * self.weight_decay * param
                    
                    # --- 4. APLIKACJA LEKU ---
                    # Po wyleczeniu zapalenia, lek stabilizuje uczenie aplikując właściwy kierunek (Dopamina/GABA)
                    param -= self.lr * (lookahead_dopamine / calmed_stress)
                    
                    # Czyszczenie żył dla kolejnej dawki
                    param.grad.zero_()

    def discharge_patient(self):
        """
        Wypisanie pacjenta. Funkcja wywoływana, gdy zamykamy sesję Continuous Learning,
        by drastycznie uciąć użycie VRAM - niszczymy maszyny dopaminy i stresu i pozwalamy Garbage Collectorowi oczyścić VRAM.
        """
        self.dopamine_memory = None
        self.stress_memory = None
        torch.cuda.empty_cache() # Fizyczne nakazanie zwolnienia pamięci GPU z maszyn podtrzymujących

if __name__ == "__main__":
    # Mały test laboratoryjny przed wdrożeniem do pacjenta Kropelki
    print("[LABORATORIUM] Inicjacja sztucznego neuronu...")
    test_model = nn.Linear(10, 2)
    # Wywołanie lekarstwa (Optymalizatora)
    iv_drip = NeuroCatechol_XR(test_model, lr=0.01)
    
    print("[LABORATORIUM] Pacjent podłączony do kroplówki NeuroCatechol_XR. Rezerwa VRAM aktywna.")
    
    # Symulacja fałszywego strzału bólowego bez używania danych
    dummy_input = torch.randn(1, 10)
    dummy_target = torch.randn(1, 2)
    criterion = nn.MSELoss()
    
    out = test_model(dummy_input)
    pain = criterion(out, dummy_target)
    
    print(f"[LABORATORIUM] Wykryto ostry stan zapalny (Błąd na poziomie: {pain.item():.4f})")
    
    # Podanie dawki próbnej
    iv_drip.inject_drug(pain)
    
    print("[LABORATORIUM] Zabieg pomyślny! Wagi zostały zoptymalizowane.")
    
    # Odpięcie
    iv_drip.discharge_patient()
    print("[LABORATORIUM] Maszyny odpięte, zwalnianie VRAM. Operacja zakończona sukcesem.")
